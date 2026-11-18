//! Exact base-G1 sequencer layouts from the pinned module descriptors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind { Interval, Note, Value, Event }
impl Kind {
    pub fn from_type(name:&str)->Option<Self>{match name{"NoteSeqA"=>Some(Self::Interval),"NoteSeqB"=>Some(Self::Note),"CtrlSeq"=>Some(Self::Value),"EventSeq"=>Some(Self::Event),_=>None}}
    pub fn name(self)->&'static str{match self{Self::Interval=>"interval",Self::Note=>"note",Self::Value=>"value",Self::Event=>"event"}}
    pub fn rows(self)->usize{if self==Self::Event{2}else{1}}
    pub fn parameter(self,row:usize,step:usize)->Option<usize>{if row>=self.rows()||step>=16{None}else{Some(if self==Self::Event{4+row*16+step}else{step})}}
    pub fn length_parameter(self)->usize{if self==Self::Event{0}else{16}}
    pub fn display(self,value:u8)->String{match self{
        Self::Interval=>format!("{:+}",i16::from(value)-64),
        Self::Note=>format!("{}{}",["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"][usize::from(value%12)],i16::from(value/12)-1),
        Self::Value=>value.to_string(),Self::Event=>if value==0{"Off"}else{"On"}.into(),
    }}
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn layouts_keep_steps_separate_from_controls(){
        for name in ["NoteSeqA","NoteSeqB","CtrlSeq"]{let kind=Kind::from_type(name).unwrap();assert_eq!(kind.parameter(0,15),Some(15));assert_eq!(kind.parameter(1,0),None);assert_eq!(kind.length_parameter(),16);}
        assert_eq!(Kind::Event.parameter(0,0),Some(4));assert_eq!(Kind::Event.parameter(1,15),Some(35));assert_eq!(Kind::Event.parameter(1,16),None);
        assert_eq!(Kind::Note.display(60),"C4");assert_eq!(Kind::Interval.display(64),"+0");assert!(Kind::from_type("OscA").is_none());
    }
}
