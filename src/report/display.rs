use super::Report;
use num_format::Buffer;
use std::fmt;

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buffer = Buffer::default();
        buffer.write_formatted(&self.total_files(), &self.locale);
        f.write_str(buffer.as_str())?;
        f.write_str(" scanned files: ")?;
        fmt::Display::fmt(&self.total_size().get_appropriate_unit(true), f)?;
        f.write_str("\n")?;
        let mut buffer = Buffer::default();
        buffer.write_formatted(&self.number_uniques, &self.locale);
        f.write_str(buffer.as_str())?;
        f.write_str(" unique files: ")?;
        fmt::Display::fmt(&self.uniques_bytes.get_appropriate_unit(true), f)?;
        f.write_str("\n")?;
        let mut groups_buffer = Buffer::default();
        groups_buffer.write_formatted(&self.duplicate_groups, &self.locale);
        let mut number_dupes_buffer = Buffer::default();
        number_dupes_buffer.write_formatted(&self.number_duplicates, &self.locale);
        f.write_str(groups_buffer.as_str())?;
        f.write_str(" groups of duplicate files (")?;
        f.write_str(number_dupes_buffer.as_str())?;
        f.write_str(" files): ")?;
        fmt::Display::fmt(&self.duplicate_bytes.get_appropriate_unit(true), f)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Report;

    #[test]
    fn empty() {
        let report = Report::default().to_string();
        let expected = "\
            0 scanned files: 0 B\n\
            0 unique files: 0 B\n\
            0 groups of duplicate files (0 files): 0 B";
        assert_eq!(report, expected);
    }

    #[test]
    #[cfg(not(windows))]
    fn basic() {
        let bag = crate::Config::builder()
            .paths(&["./tests/static"])
            .build()
            .scan::<crate::SeaHasher>();
        let report = Report::from(&bag).to_string();
        let expected = "\
            5 scanned files: 14 B\n\
            2 unique files: 5 B\n\
            1 groups of duplicate files (3 files): 9 B";
        assert_eq!(report, expected);
    }

    #[test]
    fn large_numbers_locale_en() {
        let report = Report {
            locale: num_format::Locale::en,
            number_uniques: 73_952,
            duplicate_groups: 24_583,
            number_duplicates: 137_665,
            uniques_bytes: byte_unit::Byte::from_bytes(20_293_720_473),
            duplicate_bytes: byte_unit::Byte::from_bytes(4_294_967_296),
        };
        let expected = "\
            211,617 scanned files: 22.90 GiB\n\
            73,952 unique files: 18.90 GiB\n\
            24,583 groups of duplicate files (137,665 files): 4.00 GiB";
        assert_eq!(expected, report.to_string());
    }

    #[test]
    fn large_numbers_locale_fr() {
        let report = Report {
            locale: num_format::Locale::fr,
            number_uniques: 73_952,
            duplicate_groups: 24_583,
            number_duplicates: 137_665,
            uniques_bytes: byte_unit::Byte::from_bytes(20_293_720_473),
            duplicate_bytes: byte_unit::Byte::from_bytes(4_294_967_296),
        };
        // "\u{202f}" narrow no-break space
        let expected = "\
            211\u{202f}617 scanned files: 22.90 GiB\n\
            73\u{202f}952 unique files: 18.90 GiB\n\
            24\u{202f}583 groups of duplicate files (137\u{202f}665 files): 4.00 GiB";
        assert_eq!(expected, report.to_string());
    }
}
