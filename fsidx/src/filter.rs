#[derive(Clone, Debug)]
pub enum FilterToken {
    Text(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
}
