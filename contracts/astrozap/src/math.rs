use num_bigint::BigInt;

/// The maximum number of iterations to do when solving the quadratic equation
const MAX_ITERATIONS: usize = 32;

/// Commission rate of the Astroport XYK pool, set to 0.3%
///
/// We can technically query the factory contract for this number, but this is, in my opinion,
/// unnecessary and a waste of gas because the rate is almost never going to change. If it does
/// change, we can always update this constant here and migrate the contract.
const COMMISSION_RATE_BPS: u64 = 30;

/// Equation describing the relation between the optimal swap amount (x) and the asset amounts. It
/// is a quadratic equation of the form `a * x^2 + b * x + c = 0` where `a, b, c >= 0`. For details,
/// see the document `docs/astrozap.pdf`
///
/// NOTE: Do we really need Uint512 or will Uint256 do? The maximum value we may reach is
///   (2^128 - 1)^3 ~= 1e+115
/// while the maximum amount Decimal256 supports is
///   2^256 - 1 ~= 1e+77
/// This being said, overflow should be very rare unless for pools with ultra-low-price shitcoins,
/// and even in such cases we will safely return with error
pub struct Quadratic {
    /// Coefficient of the quadratic term
    pub a: BigInt,
    /// Coefficient of the linear term
    pub b: BigInt,
    /// The constant term
    pub c: BigInt,
}

impl Quadratic {
    /// Create a new quadratic equation instance using asset amounts defined in the `docs/astrozap.pdf`
    pub fn from_asset_amounts(
        offer_user: &BigInt,
        offer_pool: &BigInt,
        ask_user: &BigInt,
        ask_pool: &BigInt,
    ) -> Self {
        let a = ask_pool + ask_user;

        // the 1st term of b
        let b1 = offer_pool * &a * 2;
        // the 2nd term of b
        let b2 = ask_pool * (offer_pool + offer_user) * COMMISSION_RATE_BPS / 10000;
        // combine the two terms
        let b = b1 - b2;

        let c = offer_pool * (offer_pool * ask_user - offer_user * ask_pool);

        Self { a, b, c }
    }

    /// Compute value of the function by the given x
    ///
    /// f(x) = a * x * x + b * x + c
    pub fn compute_value(&self, x: &BigInt) -> BigInt {
        &self.a * x * x + &self.b * x + &self.c
    }

    /// Compute value of the function by the given x
    ///
    /// f'(x) = 2 * a * x + b
    pub fn compute_deriv_value(&self, x: &BigInt) -> BigInt {
        2 * &self.a * x + &self.b
    }

    /// Solve the quadratic equation `a * x^2 + b * x + c = 0` using Newton's method
    ///
    /// If `MAX_ITERATION` is reached without convergence, we simply return the latest x value. The
    /// x value at this time does not represent the optimal swap amount, but it is fine because we
    /// will check slippage tolerance at the very end of the function call, so liquidity provisions
    /// with too big slippage will be reverted.
    /// 
    /// Also, in practice, almost all such equations converge in 4 - 5 iterations.
    pub fn solve(&self) -> BigInt {
        let mut x_prev: BigInt = 0.into();
        let mut x = x_prev.clone();

        // x_{n+1} = x_n - f(x_n) / f'(x_n)
        for _ in 0..MAX_ITERATIONS {
            let val = self.compute_value(&x_prev);
            let deriv_val = self.compute_deriv_value(&x_prev);
            x = &x_prev - val / deriv_val;
            if x == x_prev {
                break;
            } else {
                x_prev = x.clone()
            }
        }

        x
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helpers::bigint_to_uint128;
    use cosmwasm_std::Uint128;

    fn mock_equation() -> Quadratic {
        Quadratic::from_asset_amounts(
            &100000000000u128.into(),
            &118070429547232u128.into(),
            &0.into(),
            &1451993415113u128.into(),
        )
    }

    #[test]
    fn should_create_from_asset_amounts() {
        let qe = mock_equation();
        assert_eq!(qe.a, BigInt::from(1451993415113u128));
        assert_eq!(qe.b, BigInt::from(342360224387597541370186081u128));
        assert_eq!(qe.c,BigInt::from(-17143748622214425401611721600000000000i128));
    }

    #[test]
    fn should_solve_equation() {
        let qe = mock_equation();
        let offer_amount_bi = qe.solve();
        let offer_amount = bigint_to_uint128(&offer_amount_bi).unwrap();
        assert_eq!(offer_amount, Uint128::new(50064546170u128));
    }
}
