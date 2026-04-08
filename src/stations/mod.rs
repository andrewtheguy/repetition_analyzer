mod knx;

use clap::ValueEnum;

use crate::extract::Segment;

#[derive(Clone, Debug, ValueEnum)]
pub enum Station {
    Knx,
}

pub fn classify(station: &Station, seg: &Segment) -> String {
    match station {
        Station::Knx => knx::classify(seg),
    }
}
