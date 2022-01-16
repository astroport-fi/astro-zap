use cosmwasm_std::{StdError, StdResult, Uint512};

/// The maximum number of iterations to do when solving the quadratic equation
const MAX_ITERATIONS: usize = 32;

/// Commission rate of the Astroport XYK pool, set to 0.3%
///
/// We can technically query the factory contract for this number, but this is, in my opinion,
/// unnecessary and a waste of gas because the rate is almost never going to change. If it does
/// change, we can always update this constant here and migrate the contract.
const COMMISSION_RATA_BPS: u64 = 30;

/// Equation describing the relation between the optimal swap amount (x) and the asset amounts. It 
/// is a quadratic equation of the form `a * x^2 + b * x - c = 0` where `a, b, c >= 0`. For details,
/// see the document `docs/astrozap.pdf`
///
/// For our use in AstroZap, `a, b, c > 0` should always hold. If not, them something is wrong.
///
/// NOTE: Do we really need Uint512 or will Uint256 do? The maximum value we may reach is
///   (2^128 - 1)^3 ~= 1e+115
/// while the maximum amount Decimal256 supports is
///   2^256 - 1 ~= 1e+77
/// This being said, overflow should be very rare unless for pools with ultra-low-price shitcoins,
/// and even in such cases we will safely return with error
pub struct Equation {
    /// Coefficient of the quadratic term
    pub a: Uint512,
    /// Coefficient of the linear term
    pub b: Uint512,
    /// The constant term
    pub c: Uint512,
}

impl Equation {
    /// Create a new `Equation` instance using asset amounts defined in the `docs/astrozap.pdf`
    pub fn from_assets(
        offer_user: Uint512,
        offer_pool: Uint512,
        ask_user: Uint512,
        ask_pool: Uint512,
    ) -> StdResult<Self> {
        let a = ask_pool.checked_add(ask_user)?;

        // the 1st term of b
        let b_1 = offer_pool
            .checked_mul(a)?
            .checked_mul(Uint512::from(2u128))?;
        // the 2nd term of b
        let b_2 = ask_pool
            .checked_mul(offer_pool.checked_add(offer_user)?)?
            .checked_mul(Uint512::from(10000u128) - Uint512::from(COMMISSION_RATA_BPS))?
            .checked_div(Uint512::from(10000u128))?;
        // combine the two terms
        let b = b_1.checked_sub(b_2)?;

        let c = offer_pool.checked_mul(
            offer_user
                .checked_mul(ask_pool)?
                .checked_sub(offer_pool.checked_mul(ask_user)?)?,
        )?;

        Ok(Self { a, b, c })
    }

    /// Compute value of the function by the given x
    ///
    /// f(x) = a * x * x + b * x - c
    pub fn compute_value(&self, x: Uint512) -> StdResult<Uint512> {
        self.a
            .checked_mul(x)?
            .checked_mul(x)?
            .checked_add(self.b.checked_mul(x)?)?
            .checked_sub(self.c)
            .map_err(|overflow_err| StdError::overflow(overflow_err))
    }

    /// Compute value of the function by the given x
    ///
    /// f'(x) = 2 * a * x + b
    pub fn compute_derivative_value(&self, x: Uint512) -> StdResult<Uint512> {
        self.a
            .checked_mul(x)?
            .checked_mul(Uint512::from(2u128))?
            .checked_add(self.b)
            .map_err(|overflow_err| StdError::overflow(overflow_err))
    }

    /// Solve the quadratic equation `a * x^2 + b * x + c = 0` using Newton's method, with `x_init`
    /// as the initial value
    ///
    /// If `MAX_ITERATION` is reached without convergence, we simply return the latest x value. The
    /// x value at this time does not represent the optimal swap amount, but it is fine because we
    /// will check slippage tolerance at the very end of the function call, so liquidity provisions
    /// with too big slippage will be reverted.
    pub fn solve(&self, x_init: Uint512) -> StdResult<Uint512> {
        let mut x_prev = x_init;
        let mut x = x_prev;

        // x_{n+1} = x_n - f(x_n) / f'(x_n)
        for _ in 0..MAX_ITERATIONS {
            let val = self.compute_value(x_prev)?;
            let deriv_val = self.compute_derivative_value(x_prev)?;
            x = x_prev.checked_sub(val.checked_div(deriv_val)?)?;
            if x == x_prev {
                break
            } else {
                x_prev = x
            }
        }

        Ok(x)
    }
}
