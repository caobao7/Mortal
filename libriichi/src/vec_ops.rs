use std::ops::AddAssign;

/// Expecting some SIMD optimizations.
/// lhs 内的元素加上对应 rhs 内的元素
#[inline]
pub(crate) fn vec_add_assign<L, R>(lhs: &mut [L], rhs: &[R])
where
    L: Copy + AddAssign<R>,
    R: Copy,
{
    lhs.iter_mut().zip(rhs).for_each(|(l, &r)| *l += r);
}
