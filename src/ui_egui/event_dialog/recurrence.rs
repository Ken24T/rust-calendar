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
