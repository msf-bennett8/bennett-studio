//! Bennett Fibonacci 36th Codec — Rust port of PHP OrderCodeService
//! Generates temporally-sortable, human-friendly 10-char codes
//! Format: YMDHM + random (ACQPFDAQ7P)
//! A=Year(2026), C=Month(3), Q=Day(27), P=Hour(15), F=MinuteBlock(50-59), A=ExactMin(0-9), Q7P=Random

use chrono::{DateTime, Utc};
use rand::Rng;

const BASE_YEAR: i32 = 2026;
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHANUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Generate a Bennett share code (10 characters)
/// Format: [Year][Month][Day][Hour][MinBlock][ExactMin][Random4]
/// Example: ACQPFDAQ7P
pub fn generate_share_code() -> String {
    let now = Utc::now();
    generate_code_at(&now)
}

/// Generate code at a specific timestamp (deterministic for testing)
pub fn generate_code_at(dt: &DateTime<Utc>) -> String {
    let year = to_base36((dt.year() - BASE_YEAR) as usize);
    let month = to_base36((dt.month() as usize).saturating_sub(1));
    let day = to_base36_with_numbers(dt.day() as usize);
    let hour = to_base36(dt.hour() as usize);
    
    let minute = dt.minute() as usize;
    let minute_block = match minute {
        0..=9 => 'A',
        10..=19 => 'B',
        20..=29 => 'C',
        30..=39 => 'D',
        40..=49 => 'E',
        _ => 'F',
    };
    let exact_minute = to_base36(minute % 10);
    
    let random = generate_random(4);
    
    format!("{}{}{}{}{}{}{}", year, month, day, hour, minute_block, exact_minute, random)
}

/// Validate a share code format (10 chars, valid chars)
pub fn is_valid_code(code: &str) -> bool {
    if code.len() != 10 {
        return false;
    }
    code.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Extract approximate timestamp from code (for sorting/debugging)
pub fn decode_timestamp(code: &str) -> Option<DateTime<Utc>> {
    if code.len() != 10 {
        return None;
    }
    
    let chars: Vec<char> = code.chars().collect();
    let year = from_base36(chars[0])? as i32 + BASE_YEAR;
    let month = from_base36(chars[1])? as u32 + 1;
    let day = from_base36_with_numbers(chars[2])? as u32;
    let hour = from_base36(chars[3])? as u32;
    let minute_block = match chars[4] {
        'A' => 0,
        'B' => 10,
        'C' => 20,
        'D' => 30,
        'E' => 40,
        'F' => 50,
        _ => return None,
    };
    let exact_minute = from_base36(chars[5])? as u32;
    let minute = minute_block + exact_minute;
    
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(hour, minute, 0))
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

// ============================================================================
// Private helpers
// ============================================================================

fn to_base36(num: usize) -> char {
    let idx = num % 26;
    ALPHABET[idx] as char
}

fn to_base36_with_numbers(num: usize) -> char {
    let idx = num.min(35);
    ALPHANUM[idx] as char
}

fn from_base36(c: char) -> Option<usize> {
    let c = c.to_ascii_uppercase();
    ALPHABET.iter().position(|&b| b as char == c)
}

fn from_base36_with_numbers(c: char) -> Option<usize> {
    let c = c.to_ascii_uppercase();
    ALPHANUM.iter().position(|&b| b as char == c)
}

fn generate_random(length: usize) -> String {
    let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let all = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    let mut result = String::with_capacity(length);
    
    for i in 0..length {
        if i < length - 2 {
            // Prefer letters for first chars
            let idx = rng.gen_range(0..letters.len());
            result.push(letters[idx] as char);
        } else {
            // Last 2 can be alphanumeric
            let idx = rng.gen_range(0..all.len());
            result.push(all[idx] as char);
        }
    }
    
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_code_format() {
        let code = generate_share_code();
        assert_eq!(code.len(), 10);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }
    
    #[test]
    fn test_valid_code() {
        assert!(is_valid_code("ACQPFDAQ7P"));
        assert!(!is_valid_code("SHORT"));
        assert!(!is_valid_code("TOOOOOOOOOLONG"));
    }
    
    #[test]
    fn test_decode_timestamp() {
        let code = "ACQPFDAQ7P";
        let ts = decode_timestamp(code);
        assert!(ts.is_some());
    }
}
