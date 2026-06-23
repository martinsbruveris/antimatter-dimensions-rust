use super::Decimal;

#[test]
fn decimal() {
    assert_eq!(Decimal::from_float(0.0).to_string(), "0");
    assert_eq!(Decimal::from_float(f64::NAN).to_string(), "NaN");
    assert_eq!(Decimal::from_float(f64::INFINITY).to_string(), "Infinity");
    assert_eq!(
        Decimal::from_float(f64::NEG_INFINITY).to_string(),
        "-Infinity"
    );

    assert_eq!(Decimal::from_float(100.0).to_string(), "100");
    assert_eq!(Decimal::from_float(1e12).to_string(), "1000000000000");
    assert_eq!(Decimal::from_float(1.79e3).to_string(), "1790");
    assert_eq!(
        Decimal::from_float(1e308).to_string(),
        "1.0000000000000000e+308"
    );
}

#[test]
fn ops() {
    let a = Decimal { m: 3.224, e: 54 };
    let b = Decimal { m: 1.24, e: 53 };
    let c = Decimal { m: 3.1, e: 52 };

    assert_eq!(a + b, Decimal { m: 3.348, e: 54 });
    assert_eq!(a - b, Decimal { m: 3.1, e: 54 });
    assert_eq!(
        a * b,
        Decimal {
            m: 3.9977600000000004,
            e: 107
        }
    );
    assert_eq!(a / b, Decimal { m: 2.6, e: 1 });

    assert_eq!(a + c, Decimal { m: 3.255, e: 54 });
    assert_eq!(a - c, Decimal { m: 3.193, e: 54 });
    assert_eq!(a * c, Decimal { m: 9.9944, e: 106 });
    assert_eq!(a / c, Decimal { m: 1.04, e: 2 });

    assert_eq!(b + c, Decimal { m: 1.55, e: 53 });
    assert_eq!(b - c, Decimal { m: 9.3, e: 52 });
    assert_eq!(b * c, Decimal { m: 3.844, e: 105 });
    assert_eq!(
        b / c,
        Decimal {
            m: 3.9999999999999996,
            e: 0
        }
    );

    assert_eq!(
        Decimal::from_float(1.0) + Decimal::from_float(0.0),
        Decimal::from_float(1.0)
    );
}

#[test]
fn cmp() {
    let a = Decimal { m: 3.224, e: 54 };
    let b = Decimal { m: 1.24, e: 53 };
    let c = Decimal { m: 3.1, e: 52 };
    let d = Decimal { m: 3.224, e: 54 };

    assert_ne!(a, b);
    assert_eq!(a, d);
    assert_ne!(b, d);

    assert!(a >= b);
    assert!(a >= d);
    assert!(b < d);

    assert!(a > b);
    assert!(a <= d);
    assert!(b <= d);

    assert!(a > b);
    assert!(a <= d);
    assert!(b <= d);

    assert!(a >= b);
    assert!(a >= d);
    assert!(b < d);

    assert_eq!(a.max(&b), a);
    assert_eq!(a.max(&c), a);
    assert_eq!(b.max(&c), b);

    assert_eq!(a.min(&b), b);
    assert_eq!(a.min(&c), c);
    assert_eq!(b.min(&c), c);

    assert_eq!(a.clamp(&c, &b), b);
    assert_eq!(b.clamp(&c, &a), b);
    assert_eq!(c.clamp(&b, &b), b);
}

#[test]
fn neg_abs() {
    assert_eq!(-Decimal::from_float(456.7), Decimal { m: -4.567, e: 2 });
    assert_eq!(-Decimal::from_float(1.23e48), Decimal { m: -1.23, e: 48 });

    assert_eq!(
        Decimal::from_float(-456.7).abs(),
        Decimal { m: 4.567, e: 2 }
    );
    assert_eq!(
        Decimal::from_float(-1.23e48).abs(),
        Decimal { m: 1.23, e: 48 }
    );
}

#[test]
fn to_f64_roundtrip() {
    assert_eq!(Decimal::from_float(116.0).to_f64(), 116.0);
}

#[test]
fn to_str_exp() {
    assert_eq!(Decimal::from_float(0.0).to_str_exp(2), "0.00e+0");
    assert_eq!(Decimal::from_float(1.0).to_str_exp(2), "1.00e+0");
    assert_eq!(Decimal::from_float(123.456).to_str_exp(2), "1.23e+2");
    assert_eq!(Decimal::from_float(123.456).to_str_exp(5), "1.23456e+2");
    assert_eq!(Decimal::from_float(1e20).to_str_exp(3), "1.000e+20");
    assert_eq!(Decimal::from_float(-3.5).to_str_exp(1), "-3.5e+0");
    assert_eq!(Decimal::new(1.5, 1000).to_str_exp(2), "1.50e+1000");
}

#[test]
fn to_str_fixed() {
    assert_eq!(Decimal::from_float(0.0).to_str_fixed(2), "0.00");
    assert_eq!(Decimal::from_float(1.0).to_str_fixed(0), "1");
    assert_eq!(Decimal::from_float(1.0).to_str_fixed(3), "1.000");
    assert_eq!(Decimal::from_float(123.456).to_str_fixed(2), "123.46");
    assert_eq!(Decimal::from_float(123.456).to_str_fixed(0), "123");
    assert_eq!(Decimal::from_float(-3.5).to_str_fixed(1), "-3.5");
}

#[test]
fn to_str_precision() {
    assert_eq!(Decimal::from_float(123.456).to_str_precision(5), "123.46");
    assert_eq!(Decimal::from_float(123.456).to_str_precision(2), "1.2e+2");
    assert_eq!(Decimal::from_float(0.001).to_str_precision(2), "0.0010");
    assert_eq!(Decimal::from_float(1e-10).to_str_precision(2), "1.0e-10");
    assert_eq!(Decimal::from_float(1e20).to_str_precision(3), "1.00e+20");
    assert_eq!(Decimal::from_float(5.0).to_str_precision(3), "5.00");
}
