pub type Number = u128;

// Error response from actor
#[derive(Debug)]
pub struct NumError {
    pub value: Number
}

impl std::fmt::Display for NumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "value is: {}", self.value)
    }
}

impl std::error::Error for NumError {}

unsafe impl Send for NumError {}

unsafe impl Sync for NumError {}