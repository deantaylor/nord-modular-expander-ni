use crate::engine::{Action, EngineAdapter, MockEngine, Source};
use crate::project::*;
use serde_json::{json, Value};

pub struct State {
    engine: MockEngine,
    last_action: String,
}
impl State {
    pub fn new() -> Self {
        Self {
            engine: MockEngine::new(),
            last_action: "MOCK PATCH · NO ENGINE".into(),
        }
    }
    pub fn project(&self) -> &Project {
        self.engine.project()
    }
    pub fn revision(&self) -> u64 {
        self.engine.revision()
    }
    pub fn restore(&mut self, project: Project) -> Result<bool, String> {
        let changed = self.engine.replace(project)?;
        if changed {
            self.last_action = "PROJECT RESTORED · NO ENGINE".into();
        }
        Ok(changed)
    }
    fn apply(
        &mut self,
        action: Action,
        source: Source,
        revision: Option<u64>,
    ) -> Result<bool, String> {
        let changed = self.engine.apply(action, source, revision)?;
        if changed {
            self.last_action = format!("{source:?} · REV {}", self.revision());
        }
        Ok(changed)
    }
    pub fn as_json(&self) -> Value {
        let project = self.engine.project();
        let (slot, module) = project.active();
        let parameters:Vec<_>=module.parameters.iter().skip(slot.page*8).take(8).map(|p|json!({
            "id":p.id,"label":p.name,"value":p.value,"kind":p.kind,"normalized":p.normalized(),"display":p.display()
        })).collect();
        let mut labels = vec!["—".to_string(); 8];
        let mut values = vec![0.; 8];
        let mut display = vec!["—".to_string(); 8];
        for (i, p) in module
            .parameters
            .iter()
            .skip(slot.page * 8)
            .take(8)
            .enumerate()
        {
            labels[i] = p.name.clone();
            values[i] = p.normalized();
            display[i] = p.display();
        }
        json!({
            "engine":"G1 MOCK / NO ENGINE","slot":project.selected_slot,"patch":slot.patch.name,"patchId":slot.patch.id,
            "page":slot.page,"pageCount":module.pages(),"pageName":module.name,"moduleId":module.id,
            "modules":slot.patch.modules.iter().map(|m|json!({"id":m.id,"name":m.name,"pages":m.pages()})).collect::<Vec<_>>(),
            "presets":PRESETS.iter().map(|(id,name)|json!({"id":id,"name":name})).collect::<Vec<_>>(),
            "labels":labels,"values":values,"displayValues":display,"parameters":parameters,
            "revision":self.revision(),"source":self.engine.source,"status":self.last_action
        })
    }
    pub fn turn(&mut self, name: &str, delta: f64) -> bool {
        let Some(i) = encoder_index(name) else {
            return false;
        };
        self.apply(Action::Turn { index: i, delta }, Source::Hardware, None)
            .unwrap_or(false)
    }
    fn button_action(&self, name: &str) -> Option<Action> {
        let project = self.engine.project();
        let (slot, module) = project.active();
        if let Some(n) = name
            .strip_prefix("top_")
            .and_then(|n| n.parse::<usize>().ok())
        {
            return match n {
                1..=3 => Some(Action::SelectPage { page: n - 1 }),
                4 => {
                    let i = PRESETS
                        .iter()
                        .position(|(id, _)| *id == slot.patch.id)
                        .unwrap_or(0);
                    Some(Action::LoadPreset {
                        preset: PRESETS[(i + 1) % PRESETS.len()].0.into(),
                    })
                }
                5..=8 => Some(Action::SelectSlot {
                    slot: Slot::ALL[n - 5],
                }),
                _ => None,
            };
        }
        if let Some(i) = name.strip_prefix("group_").and_then(|n| {
            ["a", "b", "c", "d", "e", "f", "g", "h"]
                .iter()
                .position(|s| *s == n)
        }) {
            return slot.patch.modules.get(i).map(|m| Action::SelectModule {
                module: m.id.clone(),
            });
        }
        let module_index = slot
            .patch
            .modules
            .iter()
            .position(|m| m.id == slot.module)
            .unwrap();
        match name {
            "encoder_left" => slot
                .patch
                .modules
                .get(module_index.saturating_sub(1))
                .map(|m| Action::SelectModule {
                    module: m.id.clone(),
                }),
            "encoder_right" => slot
                .patch
                .modules
                .get((module_index + 1).min(slot.patch.modules.len() - 1))
                .map(|m| Action::SelectModule {
                    module: m.id.clone(),
                }),
            "encoder_up" => Some(Action::SelectPage {
                page: slot.page.saturating_sub(1),
            }),
            "encoder_down" => Some(Action::SelectPage {
                page: (slot.page + 1).min(module.pages() - 1),
            }),
            _ => None,
        }
    }
    pub fn button(&mut self, name: &str) -> bool {
        let Some(action) = self.button_action(name) else {
            return false;
        };
        self.apply(action, Source::Hardware, None).unwrap_or(false)
    }
    pub fn command(&mut self, command: &str) -> bool {
        let parts: Vec<_> = command.split_whitespace().collect();
        let action = match parts.as_slice() {
            ["turn", index, delta] => match (index.parse::<usize>(), delta.parse::<f64>()) {
                (Ok(i), Ok(delta)) if (1..=8).contains(&i) => Some(Action::Turn {
                    index: i - 1,
                    delta,
                }),
                _ => None,
            },
            ["button", name] => self.button_action(name),
            ["set", module, parameter, value] => {
                value.parse::<f64>().ok().map(|value| Action::SetParameter {
                    slot: self.engine.project().selected_slot,
                    module: ModuleId((*module).into()),
                    parameter: ParameterId((*parameter).into()),
                    value,
                })
            }
            ["preset", id] => Some(Action::LoadPreset {
                preset: (*id).into(),
            }),
            ["module", id] => Some(Action::SelectModule {
                module: ModuleId((*id).into()),
            }),
            ["page", page] => page
                .parse::<usize>()
                .ok()
                .filter(|n| *n > 0)
                .map(|n| Action::SelectPage { page: n - 1 }),
            _ => None,
        };
        action.is_some_and(|action| self.apply(action, Source::Terminal, None).is_ok())
    }
}
fn encoder_index(name: &str) -> Option<usize> {
    name.strip_prefix("screen_encoder_")?
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=8).contains(n))
        .map(|n| n - 1)
}
