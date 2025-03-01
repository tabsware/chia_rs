use super::sanitize_int::{sanitize_uint, SanitizedUint};
use super::validation_error::{atom, ErrorCode, ValidationErr};
use crate::gen::flags::NO_UNKNOWN_CONDS;
use clvmr::allocator::{Allocator, NodePtr};

pub fn sanitize_hash(
    a: &Allocator,
    n: NodePtr,
    size: usize,
    code: ErrorCode,
) -> Result<NodePtr, ValidationErr> {
    let buf = atom(a, n, code)?;

    if buf.as_ref().len() != size {
        Err(ValidationErr(n, code))
    } else {
        Ok(n)
    }
}

pub fn parse_amount(a: &Allocator, n: NodePtr, code: ErrorCode) -> Result<u64, ValidationErr> {
    // amounts are not allowed to exceed 2^64. i.e. 8 bytes
    match sanitize_uint(a, n, 8, code)? {
        SanitizedUint::NegativeOverflow => Err(ValidationErr(n, code)),
        SanitizedUint::PositiveOverflow => Err(ValidationErr(n, code)),
        SanitizedUint::Ok(r) => Ok(r),
    }
}

pub fn sanitize_announce_msg(
    a: &Allocator,
    n: NodePtr,
    code: ErrorCode,
) -> Result<NodePtr, ValidationErr> {
    let buf = atom(a, n, code)?;

    if buf.as_ref().len() > 1024 {
        Err(ValidationErr(n, code))
    } else {
        Ok(n)
    }
}

pub fn sanitize_message_mode(
    a: &Allocator,
    node: NodePtr,
    flags: u32,
) -> Result<u32, ValidationErr> {
    let Some(mode) = a.small_number(node) else {
        return Err(ValidationErr(node, ErrorCode::InvalidMessageMode));
    };
    // only 6 bits are allowed to be set
    if (mode & !0b111111) != 0 {
        return Err(ValidationErr(node, ErrorCode::InvalidMessageMode));
    }
    // both the sender and the receiver must commit to *something*
    // the mode flags are: parent, puzzle, amount. First for the sender, then
    // for the receiver
    if (flags & NO_UNKNOWN_CONDS) != 0 {
        // in mempool mode, we only accept message modes where at least the
        // parent or the puzzle hash is committed to, for both the sender and
        // receiver
        if (mode & 0b110) == 0 || (mode & 0b110000) == 0 {
            return Err(ValidationErr(node, ErrorCode::InvalidMessageMode));
        }
    }
    Ok(mode)
}

#[cfg(test)]
use rstest::rstest;

#[cfg(test)]
#[rstest]
#[case(0, false, true)]
#[case(-1, false, false)]
#[case(1, false, true)]
#[case(10000000000, false, false)]
#[case(0xffffffffffff, false, false)]
#[case(-0xffffffffffff, false, false)]
#[case(0b1001001, false, false)]
// committing to only amount is not allowed in mempool mode
#[case(0b001001, false, true)]
#[case(0b010010, true, true)]
#[case(0b100100, true, true)]
#[case(0b101101, true, true)]
// committing to only amount is not allowed in mempool mode
#[case(0b100001, false, true)]
#[case(0b111111, true, true)]
#[case(0b111100, true, true)]
#[case(0b100111, true, true)]
// not committing to anything is not allowed in mempool mode
#[case(0b000111, false, true)]
#[case(0b111000, false, true)]
fn test_sanitize_mode(#[case] value: i64, #[case] pass_mempool: bool, #[case] pass: bool) {
    let mut a = Allocator::new();
    let node = a.new_number(value.into()).unwrap();

    let ret = sanitize_message_mode(&a, node, 0);
    if pass {
        assert_eq!(ret.unwrap() as i64, value);
    } else {
        assert_eq!(ret.unwrap_err().1, ErrorCode::InvalidMessageMode);
    }

    let ret = sanitize_message_mode(&a, node, NO_UNKNOWN_CONDS);
    if pass_mempool {
        assert_eq!(ret.unwrap() as i64, value);
    } else {
        assert_eq!(ret.unwrap_err().1, ErrorCode::InvalidMessageMode);
    }
}

#[cfg(test)]
fn zero_vec(len: usize) -> Vec<u8> {
    let mut ret = Vec::<u8>::new();
    for _i in 0..len {
        ret.push(0);
    }
    ret
}

#[test]
fn test_sanitize_hash() {
    let mut a = Allocator::new();
    let short = zero_vec(31);
    let valid = zero_vec(32);
    let long = zero_vec(33);

    let short_n = a.new_atom(&short).unwrap();
    assert_eq!(
        sanitize_hash(&a, short_n, 32, ErrorCode::InvalidCondition),
        Err(ValidationErr(short_n, ErrorCode::InvalidCondition))
    );
    let valid_n = a.new_atom(&valid).unwrap();
    assert_eq!(
        sanitize_hash(&a, valid_n, 32, ErrorCode::InvalidCondition),
        Ok(valid_n)
    );
    let long_n = a.new_atom(&long).unwrap();
    assert_eq!(
        sanitize_hash(&a, long_n, 32, ErrorCode::InvalidCondition),
        Err(ValidationErr(long_n, ErrorCode::InvalidCondition))
    );

    let pair = a.new_pair(short_n, long_n).unwrap();
    assert_eq!(
        sanitize_hash(&a, pair, 32, ErrorCode::InvalidCondition),
        Err(ValidationErr(pair, ErrorCode::InvalidCondition))
    );
}

#[test]
fn test_sanitize_announce_msg() {
    let mut a = Allocator::new();
    let valid = zero_vec(1024);
    let valid_n = a.new_atom(&valid).unwrap();
    assert_eq!(
        sanitize_announce_msg(&a, valid_n, ErrorCode::InvalidCondition),
        Ok(valid_n)
    );

    let long = zero_vec(1025);
    let long_n = a.new_atom(&long).unwrap();
    assert_eq!(
        sanitize_announce_msg(&a, long_n, ErrorCode::InvalidCondition),
        Err(ValidationErr(long_n, ErrorCode::InvalidCondition))
    );

    let pair = a.new_pair(valid_n, long_n).unwrap();
    assert_eq!(
        sanitize_announce_msg(&a, pair, ErrorCode::InvalidCondition),
        Err(ValidationErr(pair, ErrorCode::InvalidCondition))
    );
}

#[cfg(test)]
fn amount_tester(buf: &[u8]) -> Result<u64, ValidationErr> {
    let mut a = Allocator::new();
    let n = a.new_atom(buf).unwrap();

    parse_amount(&a, n, ErrorCode::InvalidCoinAmount)
}

#[test]
fn test_sanitize_amount() {
    // negative amounts are not allowed
    assert_eq!(
        amount_tester(&[0x80]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );
    assert_eq!(
        amount_tester(&[0xff]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );
    assert_eq!(
        amount_tester(&[0xff, 0]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );

    // leading zeros are somtimes necessary to make values positive
    assert_eq!(amount_tester(&[0, 0xff]), Ok(0xff));
    // but are disallowed when they are redundant
    assert_eq!(
        amount_tester(&[0, 0, 0, 0xff]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );
    assert_eq!(
        amount_tester(&[0, 0, 0, 0x80]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );
    assert_eq!(
        amount_tester(&[0, 0, 0, 0x7f]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );
    assert_eq!(
        amount_tester(&[0, 0, 0]).unwrap_err().1,
        ErrorCode::InvalidCoinAmount
    );

    // amounts aren't allowed to be too big
    assert_eq!(
        amount_tester(&[0x7f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .unwrap_err()
            .1,
        ErrorCode::InvalidCoinAmount
    );

    // this is small enough though
    assert_eq!(
        amount_tester(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        Ok(0xffffffffffffffff)
    );
}
