
use clap::builder::PossibleValue;

#[derive(Clone, Copy, Debug)]
pub enum Order {
    Date, Name, Random, Size,
}

impl clap::ValueEnum for Order {
    fn value_variants<'a>() -> &'a [Self] {
        &[Order::Date, Order::Name, Order::Random, Order::Size]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Order::Date => PossibleValue::new("date"),
            Order::Name => PossibleValue::new("name"),
            Order::Random => PossibleValue::new("random").help("this is default"),
            Order::Size => PossibleValue::new("size"),
        })
    }
}
impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
