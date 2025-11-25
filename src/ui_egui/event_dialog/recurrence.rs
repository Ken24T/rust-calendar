use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
        }
    }

    pub fn to_rrule_freq(&self) -> &'static str {
        match self {
            Self::Daily => "DAILY",
            Self::Weekly => "WEEKLY",
            Self::Monthly => "MONTHLY",
            Self::Yearly => "YEARLY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrencePattern {
    None,
    FirstDayOfPeriod,
    LastDayOfPeriod,
    FirstWeekdayOfPeriod(Weekday),
    LastWeekdayOfPeriod(Weekday),
}

impl RecurrencePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::FirstDayOfPeriod => "First Day",
            Self::LastDayOfPeriod => "Last Day",
            Self::FirstWeekdayOfPeriod(_) => "First Weekday",
            Self::LastWeekdayOfPeriod(_) => "Last Weekday",
        }
    }

    pub fn selected_weekday(&self) -> Option<Weekday> {
        match self {
            Self::FirstWeekdayOfPeriod(day) | Self::LastWeekdayOfPeriod(day) => Some(*day),
            _ => None,
        }
    }

    pub fn with_weekday(self, weekday: Weekday) -> Self {
        match self {
            Self::FirstWeekdayOfPeriod(_) => Self::FirstWeekdayOfPeriod(weekday),
            Self::LastWeekdayOfPeriod(_) => Self::LastWeekdayOfPeriod(weekday),
            other => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }

    pub fn to_rrule_day(&self) -> &'static str {
        match self {
            Self::Sunday => "SU",
            Self::Monday => "MO",
            Self::Tuesday => "TU",
            Self::Wednesday => "WE",
            Self::Thursday => "TH",
            Self::Friday => "FR",
            Self::Saturday => "SA",
        }
    }

    pub fn from_rrule_day(day: &str) -> Option<Self> {
        match day {
            "SU" => Some(Self::Sunday),
            "MO" => Some(Self::Monday),
            "TU" => Some(Self::Tuesday),
            "WE" => Some(Self::Wednesday),
            "TH" => Some(Self::Thursday),
            "FR" => Some(Self::Friday),
            "SA" => Some(Self::Saturday),
            _ => None,
        }
    }

    pub fn from_index(index: u8) -> Option<Self> {
        match index % 7 {
            0 => Some(Self::Sunday),
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            _ => None,
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Sunday => "Sun",
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
        }
    }

    pub fn all() -> [Self; 7] {
        [
            Self::Sunday,
            Self::Monday,
            Self::Tuesday,
            Self::Wednesday,
            Self::Thursday,
            Self::Friday,
            Self::Saturday,
        ]
    }
}

pub fn parse_until_date(value: &str) -> Option<NaiveDate> {
    if value.len() < 8 {
        return None;
    }

    let year = value[0..4].parse::<i32>().ok()?;
    let month = value[4..6].parse::<u32>().ok()?;
    let day = value[6..8].parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Holds parsed RRULE data extracted from an RRULE string
#[derive(Debug, Clone)]
pub struct ParsedRRule {
    pub is_recurring: bool,
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until_date: Option<NaiveDate>,
    pub pattern: RecurrencePattern,
    pub byday_flags: [bool; 7],
}

impl Default for ParsedRRule {
    fn default() -> Self {
        Self {
            is_recurring: false,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            count: None,
            until_date: None,
            pattern: RecurrencePattern::None,
            byday_flags: [false; 7],
        }
    }
}

impl ParsedRRule {
    /// Parse an RRULE string into structured data
    pub fn parse(rrule: &str) -> Self {
        let mut result = Self {
            is_recurring: true,
            ..Default::default()
        };

        for part in rrule.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "FREQ" => {
                        result.frequency = match value {
                            "DAILY" => RecurrenceFrequency::Daily,
                            "WEEKLY" => RecurrenceFrequency::Weekly,
                            "MONTHLY" => RecurrenceFrequency::Monthly,
                            "YEARLY" => RecurrenceFrequency::Yearly,
                            _ => RecurrenceFrequency::Daily,
                        };
                    }
                    "INTERVAL" => {
                        if let Ok(val) = value.parse::<u32>() {
                            result.interval = val;
                        }
                    }
                    "COUNT" => {
                        if let Ok(val) = value.parse::<u32>() {
                            result.count = Some(val);
                        }
                    }
                    "UNTIL" => {
                        result.until_date = parse_until_date(value);
                    }
                    "BYMONTHDAY" => {
                        if value == "1" {
                            result.pattern = RecurrencePattern::FirstDayOfPeriod;
                        } else if value == "-1" {
                            result.pattern = RecurrencePattern::LastDayOfPeriod;
                        }
                    }
                    "BYDAY" => {
                        for day in value.split(',') {
                            if day.len() > 2 {
                                if day.starts_with('1') && day.len() == 3 {
                                    if let Some(weekday) = Weekday::from_rrule_day(&day[1..]) {
                                        result.pattern =
                                            RecurrencePattern::FirstWeekdayOfPeriod(weekday);
                                    }
                                } else if day.starts_with("-1") && day.len() == 4 {
                                    if let Some(weekday) = Weekday::from_rrule_day(&day[2..]) {
                                        result.pattern =
                                            RecurrencePattern::LastWeekdayOfPeriod(weekday);
                                    }
                                }
                            } else {
                                match day {
                                    "SU" => result.byday_flags[0] = true,
                                    "MO" => result.byday_flags[1] = true,
                                    "TU" => result.byday_flags[2] = true,
                                    "WE" => result.byday_flags[3] = true,
                                    "TH" => result.byday_flags[4] = true,
                                    "FR" => result.byday_flags[5] = true,
                                    "SA" => result.byday_flags[6] = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        result
    }
}

/// Input data for building an RRULE string
#[derive(Debug, Clone)]
pub struct RRuleBuilder {
    pub is_recurring: bool,
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub pattern: RecurrencePattern,
    pub byday_enabled: bool,
    pub byday_flags: [bool; 7], // [Sun, Mon, Tue, Wed, Thu, Fri, Sat]
    pub count: Option<u32>,
    pub until_date: Option<NaiveDate>,
}

impl RRuleBuilder {
    /// Build an RRULE string from the configuration
    pub fn build(&self) -> Option<String> {
        if !self.is_recurring {
            return None;
        }

        let mut parts = vec![format!("FREQ={}", self.frequency.to_rrule_freq())];

        if self.interval > 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }

        if matches!(
            self.frequency,
            RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
        ) {
            match self.pattern {
                RecurrencePattern::FirstDayOfPeriod => {
                    parts.push("BYMONTHDAY=1".to_string());
                }
                RecurrencePattern::LastDayOfPeriod => {
                    parts.push("BYMONTHDAY=-1".to_string());
                }
                RecurrencePattern::FirstWeekdayOfPeriod(weekday) => {
                    parts.push(format!("BYDAY=1{}", weekday.to_rrule_day()));
                }
                RecurrencePattern::LastWeekdayOfPeriod(weekday) => {
                    parts.push(format!("BYDAY=-1{}", weekday.to_rrule_day()));
                }
                RecurrencePattern::None => self.append_standard_byday(&mut parts),
            }
        } else if self.frequency == RecurrenceFrequency::Weekly {
            self.append_standard_byday(&mut parts);
        }

        if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        } else if let Some(until) = self.until_date {
            parts.push(format!("UNTIL={}", until.format("%Y%m%d")));
        }

        Some(parts.join(";"))
    }

    fn append_standard_byday(&self, parts: &mut Vec<String>) {
        if !self.byday_enabled {
            return;
        }

        let mut days = Vec::new();
        if self.byday_flags[0] {
            days.push("SU");
        }
        if self.byday_flags[1] {
            days.push("MO");
        }
        if self.byday_flags[2] {
            days.push("TU");
        }
        if self.byday_flags[3] {
            days.push("WE");
        }
        if self.byday_flags[4] {
            days.push("TH");
        }
        if self.byday_flags[5] {
            days.push("FR");
        }
        if self.byday_flags[6] {
            days.push("SA");
        }

        if !days.is_empty() {
            parts.push(format!("BYDAY={}", days.join(",")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rrule_daily() {
        let parsed = ParsedRRule::parse("FREQ=DAILY;INTERVAL=2");
        assert!(parsed.is_recurring);
        assert_eq!(parsed.frequency, RecurrenceFrequency::Daily);
        assert_eq!(parsed.interval, 2);
    }

    #[test]
    fn test_parse_rrule_weekly_with_byday() {
        let parsed = ParsedRRule::parse("FREQ=WEEKLY;BYDAY=MO,WE,FR");
        assert_eq!(parsed.frequency, RecurrenceFrequency::Weekly);
        assert!(parsed.byday_flags[1]); // Monday
        assert!(parsed.byday_flags[3]); // Wednesday
        assert!(parsed.byday_flags[5]); // Friday
        assert!(!parsed.byday_flags[0]); // Sunday - not set
    }

    #[test]
    fn test_parse_rrule_monthly_with_count() {
        let parsed = ParsedRRule::parse("FREQ=MONTHLY;COUNT=12");
        assert_eq!(parsed.frequency, RecurrenceFrequency::Monthly);
        assert_eq!(parsed.count, Some(12));
    }

    #[test]
    fn test_parse_rrule_with_until() {
        let parsed = ParsedRRule::parse("FREQ=YEARLY;UNTIL=20251231");
        assert_eq!(parsed.frequency, RecurrenceFrequency::Yearly);
        assert_eq!(parsed.until_date, Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()));
    }

    #[test]
    fn test_parse_rrule_first_weekday_pattern() {
        let parsed = ParsedRRule::parse("FREQ=MONTHLY;BYDAY=1MO");
        assert_eq!(parsed.pattern, RecurrencePattern::FirstWeekdayOfPeriod(Weekday::Monday));
    }

    #[test]
    fn test_parse_rrule_last_weekday_pattern() {
        let parsed = ParsedRRule::parse("FREQ=MONTHLY;BYDAY=-1FR");
        assert_eq!(parsed.pattern, RecurrencePattern::LastWeekdayOfPeriod(Weekday::Friday));
    }

    #[test]
    fn test_build_rrule_simple() {
        let builder = RRuleBuilder {
            is_recurring: true,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            pattern: RecurrencePattern::None,
            byday_enabled: false,
            byday_flags: [false; 7],
            count: None,
            until_date: None,
        };
        assert_eq!(builder.build(), Some("FREQ=DAILY".to_string()));
    }

    #[test]
    fn test_build_rrule_weekly_with_days() {
        let builder = RRuleBuilder {
            is_recurring: true,
            frequency: RecurrenceFrequency::Weekly,
            interval: 2,
            pattern: RecurrencePattern::None,
            byday_enabled: true,
            byday_flags: [false, true, false, true, false, true, false], // Mon, Wed, Fri
            count: Some(10),
            until_date: None,
        };
        assert_eq!(builder.build(), Some("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE,FR;COUNT=10".to_string()));
    }

    #[test]
    fn test_build_rrule_not_recurring() {
        let builder = RRuleBuilder {
            is_recurring: false,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            pattern: RecurrencePattern::None,
            byday_enabled: false,
            byday_flags: [false; 7],
            count: None,
            until_date: None,
        };
        assert_eq!(builder.build(), None);
    }

    #[test]
    fn test_build_rrule_monthly_first_day() {
        let builder = RRuleBuilder {
            is_recurring: true,
            frequency: RecurrenceFrequency::Monthly,
            interval: 1,
            pattern: RecurrencePattern::FirstDayOfPeriod,
            byday_enabled: false,
            byday_flags: [false; 7],
            count: None,
            until_date: None,
        };
        assert_eq!(builder.build(), Some("FREQ=MONTHLY;BYMONTHDAY=1".to_string()));
    }
}
