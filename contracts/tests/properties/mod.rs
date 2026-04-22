// Property-based tests module
// Tests will be organized by property number as defined in the design document

pub mod lmsr_properties;
pub mod state_properties;
pub mod xlm_conservation;

// Property test configuration
pub const MIN_PROPTEST_CASES: u32 = 100;
