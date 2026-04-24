/**
 * AMM Engine - Constant Product Market Maker (x * y = k)
 * Implements automated market making logic for binary prediction markets
 */

export interface AMMCalculation {
  price: number;
  shares: number;
  slippage: number;
  newProbability: number;
}

export interface LiquidityPool {
  yesPool: number;
  noPool: number;
}

/**
 * Calculate the cost to buy Yes shares using constant product formula
 * @param yesPool - Current Yes token pool size
 * @param noPool - Current No token pool size
 * @param amountIn - Amount of tokens to spend
 * @returns Calculation result with price, shares, slippage, and new probability
 */
export function calculateBuyPrice(
  yesPool: number,
  noPool: number,
  amountIn: number
): AMMCalculation {
  const k = yesPool * noPool;
  const newNoPool = noPool + amountIn;
  const newYesPool = k / newNoPool;
  const shares = yesPool - newYesPool;
  const price = amountIn / shares;
  const expectedPrice = calculateProbability(yesPool, noPool) / 100;
  const slippage = calculateSlippage(amountIn, expectedPrice, price);
  const newProbability = calculateProbability(newYesPool, newNoPool);

  return { price, shares, slippage, newProbability };
}

/**
 * Calculate the return for selling Yes shares
 * @param yesPool - Current Yes token pool size
 * @param noPool - Current No token pool size
 * @param amountOut - Number of shares to sell
 * @returns Calculation result with price, shares, slippage, and new probability
 */
export function calculateSellPrice(
  yesPool: number,
  noPool: number,
  amountOut: number
): AMMCalculation {
  const k = yesPool * noPool;
  const newYesPool = yesPool + amountOut;
  const newNoPool = k / newYesPool;
  const tokensReceived = noPool - newNoPool;
  const price = tokensReceived / amountOut;
  const expectedPrice = calculateProbability(yesPool, noPool) / 100;
  const slippage = calculateSlippage(tokensReceived, expectedPrice, price);
  const newProbability = calculateProbability(newYesPool, newNoPool);

  return { price, shares: tokensReceived, slippage, newProbability };
}

/**
 * Calculate implied probability from pool sizes
 * @param yesPool - Current Yes token pool size
 * @param noPool - Current No token pool size
 * @returns Probability as percentage (0-100)
 */
export function calculateProbability(yesPool: number, noPool: number): number {
  const total = yesPool + noPool;
  return (noPool / total) * 100;
}

/**
 * Calculate slippage percentage
 * @param amountIn - Amount of tokens spent
 * @param expectedPrice - Expected price per share
 * @param actualPrice - Actual price per share
 * @returns Slippage as percentage
 */
export function calculateSlippage(
  amountIn: number,
  expectedPrice: number,
  actualPrice: number
): number {
  if (expectedPrice === 0) return 0;
  return Math.abs(((actualPrice - expectedPrice) / expectedPrice) * 100);
}
