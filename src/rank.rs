#[derive(Clone,Copy,PartialEq,Debug)]
pub enum Rank {
   THREE_STARS, TWO_STARS, ONE_STAR, NO_STAR, 
}

impl Rank {
    pub fn from_usize(v: usize) -> Self {
        match v {
            0 => Rank::THREE_STARS,
            1 => Rank::TWO_STARS,
            2 => Rank::ONE_STAR,
            _ => Rank::NO_STAR,
        }
    }

}
