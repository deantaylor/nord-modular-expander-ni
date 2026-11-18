//! Authoritative mock reducer. It has no synthesis or Nord protocol support.
use crate::project::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Hardware,
    Terminal,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    SetParameter {
        slot: Slot,
        module: ModuleId,
        parameter: ParameterId,
        value: f64,
    },
    Turn {
        index: usize,
        delta: f64,
    },
    SelectSlot {
        slot: Slot,
    },
    SelectModule {
        module: ModuleId,
    },
    SelectPage {
        page: usize,
    },
    LoadPreset {
        preset: String,
    },
}
pub trait EngineAdapter {
    fn project(&self) -> &Project;
    fn revision(&self) -> u64;
    fn apply(&mut self, action: Action, source: Source, base: Option<u64>) -> Result<bool, String>;
}
pub struct MockEngine {
    project: Project,
    revision: u64,
    pub source: Source,
    // Discrete parameters accumulate fine encoder movement instead of dropping substeps.
    residual: HashMap<String, f64>,
}
impl MockEngine {
    pub fn new() -> Self {
        Self {
            project: Project::initial(),
            revision: 0,
            source: Source::Terminal,
            residual: HashMap::new(),
        }
    }
    pub fn replace(&mut self, project: Project) -> Result<bool, String> {
        project.validate()?;
        if self.project == project {
            return Ok(false);
        }
        self.ensure_revision()?;
        self.project = project;
        self.residual.clear();
        self.changed(Source::Terminal);
        Ok(true)
    }
    fn ensure_revision(&self) -> Result<(), String> {
        if self.revision == u64::MAX {
            Err("Revision exhausted; restart session".into())
        } else {
            Ok(())
        }
    }
    fn changed(&mut self, source: Source) {
        self.revision += 1;
        self.source = source;
    }
    fn key(slot: Slot, module: &ModuleId, parameter: &ParameterId) -> String {
        format!("{slot:?}/{}/{}", module.0, parameter.0)
    }
}
impl EngineAdapter for MockEngine {
    fn project(&self) -> &Project {
        &self.project
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn apply(&mut self, action: Action, source: Source, base: Option<u64>) -> Result<bool, String> {
        if base.is_some_and(|revision| revision != self.revision) {
            return Err("Stale state revision; refresh and retry the update".into());
        }
        self.ensure_revision()?;
        let changed = match action {
            Action::SelectSlot { slot } => {
                let changed = self.project.selected_slot != slot;
                self.project.selected_slot = slot;
                changed
            }
            Action::SelectModule { module } => {
                let slot = &mut self.project.slots[self.project.selected_slot.index()];
                if !slot.patch.modules.iter().any(|m| m.id == module) {
                    return Err("Module not found".into());
                }
                let changed = slot.module != module;
                slot.module = module;
                if changed {
                    slot.page = 0;
                }
                changed
            }
            Action::SelectPage { page } => {
                let (slot, module) = self.project.active();
                if page >= module.pages() {
                    return Err("Page not found".into());
                }
                let changed = slot.page != page;
                self.project.slots[self.project.selected_slot.index()].page = page;
                changed
            }
            Action::LoadPreset { preset: id } => {
                let patch = preset(&id)?;
                let module = patch.modules[0].id.clone();
                let replacement = SlotState {
                    patch,
                    module,
                    page: 0,
                };
                let slot = &mut self.project.slots[self.project.selected_slot.index()];
                let changed = *slot != replacement;
                *slot = replacement;
                self.residual.clear();
                changed
            }
            Action::SetParameter {
                slot,
                module,
                parameter,
                value,
            } => {
                let p = self.project.slots[slot.index()]
                    .patch
                    .modules
                    .iter_mut()
                    .find(|m| m.id == module)
                    .and_then(|m| m.parameters.iter_mut().find(|p| p.id == parameter))
                    .ok_or("Parameter not found")?;
                p.check_value(value)?;
                let changed = p.value != value;
                p.value = value;
                self.residual.remove(&Self::key(slot, &module, &parameter));
                changed
            }
            Action::Turn { index, delta } => {
                if index >= 8 || !delta.is_finite() {
                    return Err("Invalid encoder input".into());
                }
                if delta == 0.0 {
                    return Ok(false);
                }
                let slot_id = self.project.selected_slot;
                let slot = &mut self.project.slots[slot_id.index()];
                let module = slot
                    .patch
                    .modules
                    .iter_mut()
                    .find(|m| m.id == slot.module)
                    .unwrap();
                let Some(p) = module.parameters.get_mut(slot.page * 8 + index) else {
                    return Ok(false);
                };
                let key = Self::key(slot_id, &module.id, &p.id);
                let value = if matches!(p.kind, Kind::Continuous { .. }) {
                    p.value_at(p.normalized() + delta)
                } else {
                    let (min, max) = p.bounds();
                    let residue = self.residual.entry(key).or_default();
                    // Limit extreme deltas before adding residual to avoid overflow.
                    let movement =
                        (delta.clamp(-1.0, 1.0) * (max - min) + *residue).clamp(-1e6, 1e6);
                    let steps = movement.trunc();
                    let value = (p.value + steps).clamp(min, max);
                    *residue = if value == min && movement < 0.0 || value == max && movement > 0.0 {
                        0.0
                    } else {
                        movement - steps
                    };
                    value
                };
                let changed = p.value != value;
                p.value = value;
                changed
            }
        };
        if changed {
            self.changed(source);
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edit(slot: Slot, value: f64) -> Action {
        Action::SetParameter {
            slot,
            module: ModuleId("filter".into()),
            parameter: ParameterId("cutoff".into()),
            value,
        }
    }
    #[test]
    fn presets_are_valid_and_slots_are_independent() {
        for (id, _) in PRESETS {
            let mut p = Project::initial();
            p.slots[0].patch = preset(id).unwrap();
            p.validate().unwrap();
        }
        let mut e = MockEngine::new();
        e.apply(edit(Slot::B, 2000.), Source::Terminal, Some(0))
            .unwrap();
        assert_eq!(
            e.project.slots[0].patch.modules[1].parameters[0].value,
            1050.
        );
        assert_eq!(
            e.project.slots[1].patch.modules[1].parameters[0].value,
            2000.
        );
    }
    #[test]
    fn logarithmic_scaling_integer_residuals_and_enum_limits() {
        let mut e = MockEngine::new();
        let expected = 1050. * 1000_f64.powf(0.1);
        e.apply(
            Action::Turn {
                index: 0,
                delta: 0.1,
            },
            Source::Hardware,
            None,
        )
        .unwrap();
        assert!((e.project.active().1.parameters[0].value - expected).abs() < 1e-6);
        e.apply(
            Action::SelectModule {
                module: ModuleId("osc".into()),
            },
            Source::Hardware,
            None,
        )
        .unwrap();
        for _ in 0..150 {
            e.apply(
                Action::Turn {
                    index: 4,
                    delta: 0.001,
                },
                Source::Hardware,
                None,
            )
            .unwrap();
        }
        assert_eq!(e.project.active().1.parameters[4].value, 2.);
        e.apply(
            Action::Turn {
                index: 1,
                delta: 100.,
            },
            Source::Hardware,
            None,
        )
        .unwrap();
        assert_eq!(e.project.active().1.parameters[1].value, 3.);
        e.project.validate().unwrap();
    }
    #[test]
    fn parameter_updates_and_hardware_are_revisioned_and_idempotent() {
        let mut e = MockEngine::new();
        assert!(e
            .apply(edit(Slot::A, 1500.), Source::Terminal, Some(0))
            .unwrap());
        assert_eq!(e.revision(), 1);
        assert_eq!(e.source, Source::Terminal);
        assert!(e
            .apply(
                Action::Turn {
                    index: 0,
                    delta: 0.01
                },
                Source::Hardware,
                None
            )
            .unwrap());
        let value = e.project.active().1.parameters[0].value;
        let project = e.project.clone();
        assert!(e
            .apply(edit(Slot::A, 2000.), Source::Terminal, Some(1))
            .is_err());
        assert_eq!(e.project, project);
        assert!(!e
            .apply(edit(Slot::A, value), Source::Terminal, Some(2))
            .unwrap());
        assert_eq!(e.revision(), 2);
        assert_eq!(e.source, Source::Hardware);
    }
    #[test]
    fn invalid_values_and_metadata_leave_project_unchanged() {
        let mut e = MockEngine::new();
        let before = e.project.clone();
        for value in [f64::NAN, f64::INFINITY, -1., 20001.] {
            assert!(e
                .apply(edit(Slot::A, value), Source::Terminal, None)
                .is_err());
        }
        let action = Action::SetParameter {
            slot: Slot::A,
            module: ModuleId("filter".into()),
            parameter: ParameterId("mode".into()),
            value: 1.5,
        };
        assert!(e.apply(action, Source::Terminal, None).is_err());
        assert_eq!(e.project, before);
        assert_eq!(e.revision(), 0);
        let mut invalid = before.clone();
        invalid.slots[0].patch.modules[1].parameters[0].kind = Kind::Continuous {
            min: 0.,
            max: 10.,
            scale: Scale::Log,
            unit: "Hz".into(),
        };
        assert!(e.replace(invalid).is_err());
        assert_eq!(e.project, before);
    }
    #[test]
    fn dynamic_pages_partial_page_and_preset_navigation() {
        let mut e = MockEngine::new();
        e.apply(
            Action::SelectModule {
                module: ModuleId("osc".into()),
            },
            Source::Hardware,
            None,
        )
        .unwrap();
        e.apply(Action::SelectPage { page: 1 }, Source::Hardware, None)
            .unwrap();
        assert!(!e
            .apply(
                Action::Turn {
                    index: 7,
                    delta: 0.1
                },
                Source::Hardware,
                None
            )
            .unwrap());
        assert!(e
            .apply(Action::SelectPage { page: 2 }, Source::Hardware, None)
            .is_err());
        e.apply(
            Action::LoadPreset {
                preset: "step_bass".into(),
            },
            Source::Terminal,
            None,
        )
        .unwrap();
        assert_eq!(e.project.active().0.page, 0);
        assert_eq!(e.project.active().0.patch.modules.len(), 3);
        assert_eq!(e.project.slots[1].patch.id, "init");
    }
    #[test]
    fn rapid_turns_and_page_switches_remain_valid() {
        let mut e = MockEngine::new();
        for i in 0..2000 {
            e.apply(
                Action::Turn {
                    index: i % 8,
                    delta: if i % 3 == 0 { -0.01 } else { 0.01 },
                },
                Source::Hardware,
                None,
            )
            .unwrap();
            if i % 17 == 0 {
                e.apply(
                    Action::SelectSlot {
                        slot: Slot::ALL[(i / 17) % 4],
                    },
                    Source::Hardware,
                    None,
                )
                .unwrap();
            }
        }
        e.project.validate().unwrap();
    }
}
