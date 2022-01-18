import BN from "bn.js";

type BigNumberish = number | string | BN;

/**
 * TypeScript implementation of the `Equation` struct defined in `astrozap/src/math.rs`
 *
 * Represents a quadratic equation of form `a * x^2 + b * x - c = 0`
 */
export class Equation {
  MAX_ITERATIONS = 32;
  a: BN;
  b: BN;
  c: BN;

  /**
   * Create a new `Equation` instance using asset amounts
   *
   * See `docs/astrozap.pdf` for explaination
   */
  constructor(
    offer_user: BigNumberish,
    offer_pool: BigNumberish,
    ask_user: BigNumberish,
    ask_pool: BigNumberish
  ) {
    offer_user = new BN(offer_user);
    offer_pool = new BN(offer_pool);
    ask_user = new BN(ask_user);
    ask_pool = new BN(ask_pool);

    this.a = ask_pool.add(ask_user);

    let b1 = offer_pool.mul(this.a).mul(new BN(2));
    let b2 = ask_pool.mul(offer_pool.add(offer_user)).mul(new BN(30)).div(new BN(10000));
    this.b = b1.sub(b2);

    this.c = offer_pool.mul(offer_pool.mul(ask_user).sub(offer_user.mul(ask_pool)));
  }

  /**
   * Print the paramters a, b, c
   */
  parameters() {
    console.log("a =", this.a.toString());
    console.log("b =", this.b.toString());
    console.log("c =", this.c.toString());
  }

  /**
   * Compute the value of `f(x) = a * x^2 + b * x - c` at the given x
   */
  computeValue(x: BigNumberish) {
    x = new BN(x);
    return this.a.mul(x).mul(x).add(this.b.mul(x)).add(this.c);
  }

  /**
   * Compute the value of `f'(x) = 2 * a * x + b` at the given x
   */
  computeDerivValue(x: BigNumberish) {
    x = new BN(x);
    return new BN(2).mul(this.a).mul(x).add(this.b);
  }

  /**
   * Solve the equation `f(x) = 0` using Newton's method with the given initial value
   */
  solve() {
    let xPrev = new BN(0);
    let x = xPrev;

    // x_{n+1} = x_n - f(x_n) / f'(x_n)
    for (let i = 0; i < this.MAX_ITERATIONS; i++) {
      let val = this.computeValue(x);
      let derivVal = this.computeDerivValue(x);
      x = xPrev.sub(val.div(derivVal));

      console.log(
        `iteration ${i + 1}, x = ${x.toString()}, val = ${val.toString()}, derivVal = ${derivVal.toString()}`
      );

      if (x.toNumber() == xPrev.toNumber()) {
        break;
      } else {
        xPrev = x;
      }
    }

    return x.toString();
  }
}
