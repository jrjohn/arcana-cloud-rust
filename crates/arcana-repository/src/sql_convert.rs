//! Conversions between Rust integer types and the signed `BIGINT` values
//! exchanged with `MySQL`.

/// Converts a `LIMIT` / `OFFSET` value to the `i64` bound in SQL.
///
/// Values above `i64::MAX` saturate: an offset that large is past any real
/// table and still yields an empty page, instead of wrapping to a negative
/// number that `MySQL` rejects.
pub(crate) fn to_sql_bigint(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Converts a `COUNT(*)` result to `u64`.
///
/// `COUNT(*)` is never negative; a negative value is treated as 0 rather than
/// wrapping to a huge count.
pub(crate) fn count_to_u64(count: i64) -> u64 {
    u64::try_from(count).unwrap_or(0)
}
