#[macro_export]
macro_rules! impl_from {
    ($from_type:ty) => {
        impl From<$from_type> for Decimal {
            fn from(num: $from_type) -> Decimal {
                Decimal::from_float(num as f64)
            }
        }
    };
}

/// Generates a forwarding implementation of an assignment operator for a reference RHS.
/// Given `impl OpAssign<Rhs> for Lhs`, this generates:
/// - `impl OpAssign<&Rhs> for Lhs`
///
/// `Rhs` must be `Copy`.
macro_rules! forward_ref_op_assign {
    (impl $op:ident<$rhs:ty> for $lhs:ty, $method:ident) => {
        impl $op<&$rhs> for $lhs {
            fn $method(&mut self, rhs: &$rhs) {
                $op::$method(self, *rhs);
            }
        }
    };
}

/// Generates the three forwarding implementations of a binary operator for reference
/// combinations. Given `impl Op<Rhs> for Lhs`, this generates:
/// - `impl Op<&Rhs> for Lhs`
/// - `impl Op<Rhs> for &Lhs`
/// - `impl Op<&Rhs> for &Lhs`
///
/// Both `Lhs` and `Rhs` must be `Copy`.
macro_rules! forward_ref_binop {
    (impl $op:ident<$rhs:ty> for $lhs:ty, $method:ident) => {
        impl $op<&$rhs> for $lhs {
            type Output = <$lhs as $op<$rhs>>::Output;

            fn $method(self, rhs: &$rhs) -> Self::Output {
                $op::$method(self, *rhs)
            }
        }

        impl $op<$rhs> for &$lhs {
            type Output = <$lhs as $op<$rhs>>::Output;

            fn $method(self, rhs: $rhs) -> Self::Output {
                $op::$method(*self, rhs)
            }
        }

        impl $op<&$rhs> for &$lhs {
            type Output = <$lhs as $op<$rhs>>::Output;

            fn $method(self, rhs: &$rhs) -> Self::Output {
                $op::$method(*self, *rhs)
            }
        }
    };
}
