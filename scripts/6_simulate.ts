import { LCDClient } from "@terra-money/terra.js";

const ASTROZAP_ADDR = "terra1s7wn47w5kcvn8yefj8y2ffev62a3yureaz4lvc";
const ASTRO_UST_PAIR = "terra1ec0fnjk2u6mms05xyyrte44jfdgdaqnx0upesr";

type Asset = {
  info: { cw20: string } | { native: string };
  amount: string;
};

type SimulateEnterResponse = {
  offer_asset: Asset;
  return_asset: Asset;
  mint_shares: Asset;
};

const terra = new LCDClient({
  URL: "https://bombay-lcd.terra.dev",
  chainID: "bombay-12",
  gasPrices: "0.155uusd",
  gasAdjustment: 1.4,
});

(async () => {
  process.stdout.write("querying simulation... ");
  const response: SimulateEnterResponse = await terra.wasm.contractQuery(ASTROZAP_ADDR, {
    simulate_enter: {
      pair: ASTRO_UST_PAIR,
      deposits: [
        {
          info: {
            native: "uusd",
          },
          amount: "10000000",
        },
      ],
    },
  });
  console.log("done! response:", response);
  process.exit(0);
})();
