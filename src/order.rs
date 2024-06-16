
use clap::builder::PossibleValue;

#[derive(Clone, Copy, Debug)]
pub enum Order {
    Colors, Date, Name, Random, Size, Value, Palette, Label,
}

impl Order {
    pub fn from_options(name: bool, date: bool, size: bool, colors: bool, value: bool, palette: bool, label: bool) -> Self {
        if name {
            Self::Name
        } else if date {
            Self::Date
        } else if size {
            Self::Size
        } else if colors {
            Self::Colors
        } else if value {
            Self::Value
        } else if palette {
            Self::Palette
        } else if label {
            Self::Label
        } else {
            Self::Random
        }
    }
}

impl clap::ValueEnum for Order {
    fn value_variants<'a>() -> &'a [Self] {
        &[Order::Colors, Order::Date, Order::Name, Order::Random, Order::Size, Order::Value, Order::Palette, Order::Label]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Order::Colors => PossibleValue::new("colorsize"),
            Order::Date => PossibleValue::new("date"),
            Order::Name => PossibleValue::new("name"),
            Order::Random => PossibleValue::new("random").help("this is default"),
            Order::Value => PossibleValue::new("value"),
            Order::Size => PossibleValue::new("size"),
            Order::Palette => PossibleValue::new("palette"),
            Order::Label => PossibleValue::new("label"),
        })
    }
}
impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
