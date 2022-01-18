import BN from "bn.js";

const DECIMAL_FRACTIONAL = new BN("1000000000000000000");

type BigNumberish = BN | number | string;

/**
 * @notice Calculate the output when swapping in an XY-K pool
 */
export function computeXykSwapOutput(
  offerAmount: BigNumberish,
  offerDepth: BigNumberish,
  askDepth: BigNumberish
) {
  offerAmount = new BN(offerAmount);
  offerDepth = new BN(offerDepth);
  askDepth = new BN(askDepth);

  // ask_amount = (ask_pool - cp / (offer_pool + offer_amount))
  //
  // NOTE: 
  // 1. when calculating `afterDepthAfter`, Astroport first multiplies `DECIMAL_FRACTIONAL` then
  // divides in the end to offer more precision
  // 2. we assume a 0.3% commission rate
  const cp = offerDepth.mul(askDepth);
  console.log("cp:", cp.toString());
  const offerDepthAfter = offerDepth.add(offerAmount);
  const askDepthAfter = cp.mul(DECIMAL_FRACTIONAL).div(offerDepthAfter);
  const returnAmount = askDepth.mul(DECIMAL_FRACTIONAL).sub(askDepthAfter).div(DECIMAL_FRACTIONAL);
  console.log('return amount:', returnAmount.toString());

  // commission rate = 0.3%
  const commission = returnAmount.mul(new BN(30)).div(new BN(10000));
  console.log('commission amount:', commission.toString());

  // Note: return amount is after deducting commission but before duducting tax
  const returnAmountAfterFee = returnAmount.sub(commission);

  return returnAmountAfterFee.toString();
}