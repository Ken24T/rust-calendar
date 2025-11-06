// Example unit test for Frequency enum
// This demonstrates the testing pattern for the recurrence frequency types

use test_case::test_case;

// Note: This is a placeholder. Actual implementation will come from src/models/recurrence/frequency.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Yearly,
}

impl Frequency {
    pub fn to_interval(&self) -> (String, u32) {
        match self {
            Frequency::Daily => ("DAILY".to_string(), 1),
            Frequency::Weekly => ("WEEKLY".to_string(), 1),
            Frequency::Fortnightly => ("WEEKLY".to_string(), 2),
            Frequency::Monthly => ("MONTHLY".to_string(), 1),
            Frequency::Quarterly => ("MONTHLY".to_string(), 3),
            Frequency::Yearly => ("YEARLY".to_string(), 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_daily_frequency_has_interval_one() {
        let freq = Frequency::Daily;
        let (_, interval) = freq.to_interval();
        assert_eq!(interval, 1, "Daily frequency should have interval of 1");
    }
    
    #[test]
    fn test_weekly_frequency_has_interval_one() {
        let freq = Frequency::Weekly;
        let (_, interval) = freq.to_interval();
        assert_eq!(interval, 1, "Weekly frequency should have interval of 1");
    }
    
    #[test]
    fn test_fortnightly_frequency_has_interval_two() {
        let freq = Frequency::Fortnightly;
        let (rrule_freq, interval) = freq.to_interval();
        assert_eq!(rrule_freq, "WEEKLY", "Fortnightly should use WEEKLY frequency");
        assert_eq!(interval, 2, "Fortnightly should have interval of 2 (bi-weekly)");
    }
    
    #[test]
    fn test_monthly_frequency_has_interval_one() {
        let freq = Frequency::Monthly;
        let (_, interval) = freq.to_interval();
        assert_eq!(interval, 1, "Monthly frequency should have interval of 1");
    }
    
    #[test]
    fn test_quarterly_frequency_has_interval_three() {
        let freq = Frequency::Quarterly;
        let (rrule_freq, interval) = freq.to_interval();
        assert_eq!(rrule_freq, "MONTHLY", "Quarterly should use MONTHLY frequency");
        assert_eq!(interval, 3, "Quarterly should have interval of 3 months");
    }
    
    #[test]
    fn test_yearly_frequency_has_interval_one() {
        let freq = Frequency::Yearly;
        let (_, interval) = freq.to_interval();
        assert_eq!(interval, 1, "Yearly frequency should have interval of 1");
    }
    
    // Parameterized test using test-case
    #[test_case(Frequency::Daily, "DAILY", 1; "daily frequency")]
    #[test_case(Frequency::Weekly, "WEEKLY", 1; "weekly frequency")]
    #[test_case(Frequency::Fortnightly, "WEEKLY", 2; "fortnightly frequency")]
    #[test_case(Frequency::Monthly, "MONTHLY", 1; "monthly frequency")]
    #[test_case(Frequency::Quarterly, "MONTHLY", 3; "quarterly frequency")]
    #[test_case(Frequency::Yearly, "YEARLY", 1; "yearly frequency")]
    fn test_frequency_to_interval_parameterized(
        freq: Frequency,
        expected_rrule: &str,
        expected_interval: u32,
    ) {
        let (rrule_freq, interval) = freq.to_interval();
        assert_eq!(rrule_freq, expected_rrule);
        assert_eq!(interval, expected_interval);
    }
    
    #[test]
    fn test_frequency_equality() {
        assert_eq!(Frequency::Fortnightly, Frequency::Fortnightly);
        assert_ne!(Frequency::Fortnightly, Frequency::Weekly);
        assert_ne!(Frequency::Quarterly, Frequency::Monthly);
    }
    
    #[test]
    fn test_frequency_debug_formatting() {
        let freq = Frequency::Fortnightly;
        let debug_str = format!("{:?}", freq);
        assert_eq!(debug_str, "Fortnightly");
    }
}
