#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct State {
    state: u8,
}
pub const STATES: usize = 12;

const LIT_STATES: u8 = 7;
const LIT_LIT: u8 = 0;
const _MATCH_LIT_LIT: u8 = 1;
const _REP_LIT_LIT: u8 = 2;
const SHORTREP_LIT_LIT: u8 = 3;
const _MATCH_LIT: u8 = 4;
const _REP_LIT: u8 = 5;
const _SHORTREP_LIT: u8 = 6;
const LIT_MATCH: u8 = 7;
const LIT_LONGREP: u8 = 8;
const LIT_SHORTREP: u8 = 9;
const NONLIT_MATCH: u8 = 10;
const NONLIT_REP: u8 = 11;

impl State {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn reset(&mut self) {
        self.state = LIT_LIT;
    }

    pub fn get(&self) -> u8 {
        self.state
    }

    pub fn set(&mut self, other: State) {
        self.state = other.state;
    }

    pub fn update_literal(&mut self) {
        if self.state <= SHORTREP_LIT_LIT {
            self.state = LIT_LIT;
        } else if self.state <= LIT_SHORTREP {
            self.state -= 3;
        } else {
            self.state -= 6;
        }
    }

    pub fn update_match(&mut self) {
        self.state = if self.state < LIT_STATES {
            LIT_MATCH
        } else {
            NONLIT_MATCH
        };
    }

    pub fn update_long_rep(&mut self) {
        self.state = if self.state < LIT_STATES {
            LIT_LONGREP
        } else {
            NONLIT_REP
        };
    }

    pub fn update_short_rep(&mut self) {
        self.state = if self.state < LIT_STATES {
            LIT_SHORTREP
        } else {
            NONLIT_REP
        };
    }

    pub fn is_literal(&self) -> bool {
        return self.state < LIT_STATES;
    }
}

impl From<u8> for State {
    fn from(s: u8) -> Self {
        Self { state: s }
    }
}
