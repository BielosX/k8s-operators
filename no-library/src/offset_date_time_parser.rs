use std::num::NonZeroU8;
use time::error::{Format, Parse};
use time::format_description::well_known::iso8601::{Config, EncodedConfig, TimePrecision};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

/*
   Kubernetes MicroTime
   MicroTime is version of Time with microsecond level precision
*/
const CONFIG: EncodedConfig = Config::DEFAULT
    .set_time_precision(TimePrecision::Second {
        decimal_digits: NonZeroU8::new(6),
    })
    .encode();

const ISO8601: Iso8601<CONFIG> = Iso8601::<CONFIG>;

pub fn parse(input: &str) -> Result<OffsetDateTime, Parse> {
    OffsetDateTime::parse(input, &ISO8601)
}

pub fn format(datetime: OffsetDateTime) -> Result<String, Format> {
    datetime.format(&ISO8601)
}
