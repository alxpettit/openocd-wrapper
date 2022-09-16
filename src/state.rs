use std::fmt;

#[derive(Clone)]
pub(crate) enum ArgMode {
    None,
    Picoprobe,
    GeneralRp2040
}

impl std::fmt::Debug for ArgMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArgMode::Picoprobe => write!(f, "ArgMode::Picoprobe"),
            ArgMode::None => write!(f, "ArgMode::None"),
            ArgMode::GeneralRp2040 => write!(f, "ArgMode::GeneralRp2040")
        }
    }
}