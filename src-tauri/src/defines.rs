use std::str::FromStr;

#[derive(PartialEq)]
pub enum QTSOSCType {
    PUSH,
    HIT
}

impl FromStr for QTSOSCType {

    type Err = ();

    fn from_str(input: &str) -> Result<QTSOSCType, Self::Err> {
        match input {
            "PUSH" => Ok(QTSOSCType::PUSH),
            "HIT" => Ok(QTSOSCType::HIT),
            _ => Err(())
        }
    }

} // Stack Overflow goodness

#[derive(PartialEq)]
pub enum QTSInteraction {
    SHOCK,
    VIBRATE,
    BEEP
}

impl FromStr for QTSInteraction {

    type Err = ();

    fn from_str(input: &str) -> Result<QTSInteraction, Self::Err> {
        match input {
            "SHOCK" => Ok(QTSInteraction::SHOCK),
            "VIBRATE" => Ok(QTSInteraction::VIBRATE),
            "BEEP" => Ok(QTSInteraction::BEEP),
            _ => Err(())
        }
    }

}