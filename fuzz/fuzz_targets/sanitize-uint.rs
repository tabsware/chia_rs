#![no_main]
use libfuzzer_sys::fuzz_target;

use clvmr::allocator::Allocator;
use chia::gen::sanitize_int::{sanitize_uint, SanitizedUint};
use chia::gen::validation_error::{ErrorCode, ValidationErr};

fuzz_target!(|data: &[u8]| {
    let mut a = Allocator::new();
    let atom = a.new_atom(data).unwrap();
    match sanitize_uint(&a, atom, 8, ErrorCode::InvalidCoinAmount) {
        Ok(SanitizedUint::Ok(_)) => {
            assert!(data.len() <= 9);
            if data.len() == 9 {
                assert!(data[0] == 0);
            }
        },
        Ok(SanitizedUint::NegativeOverflow) => {
            assert!(data.len() > 0 && (data[0] & 0x80) != 0);
        },
        Ok(SanitizedUint::PositiveOverflow) => {
            assert!(data.len() > 8);
        },
        Err(ValidationErr(n, c)) => {
            assert!(n == atom);
            assert!(c == ErrorCode::InvalidCoinAmount);
        }
    }

    match sanitize_uint(&a, atom, 4, ErrorCode::InvalidCoinAmount) {
        Ok(SanitizedUint::Ok(_)) => {
            assert!(data.len() <= 5);
            if data.len() == 5 {
                assert!(data[0] == 0);
            }
        },
        Ok(SanitizedUint::NegativeOverflow) => {
            assert!(data.len() > 0 && (data[0] & 0x80) != 0);
        },
        Ok(SanitizedUint::PositiveOverflow) => {
            assert!(data.len() > 4);
        },
        Err(ValidationErr(n, c)) => {
            assert!(n == atom);
            assert!(c == ErrorCode::InvalidCoinAmount);
        }
    }
});
