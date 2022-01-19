import dotenv from "dotenv";
import { LCDClient, MnemonicKey, MsgExecuteContract, Wallet } from "@terra-money/terra.js";
import { waitUntilKeypress, sendTransaction } from "./3_helpers";

const ASTROZAP_ADDR = "terra1s7wn47w5kcvn8yefj8y2ffev62a3yureaz4lvc";
const ASTRO_UST_PAIR = "terra1ec0fnjk2u6mms05xyyrte44jfdgdaqnx0upesr";

const terra = new LCDClient({
  URL: "https://bombay-lcd.terra.dev",
  chainID: "bombay-12",
  gasPrices: "0.155uusd",
  gasAdjustment: 1.4,
});

let user: Wallet;
dotenv.config();
if (!process.env.MNEMONIC) {
  throw new Error("mnemonic not provided");
} else {
  user = terra.wallet(
    new MnemonicKey({
      mnemonic: process.env.MNEMONIC,
    }),
  );
}

(async () => {
  process.stdout.write("ready to submit tx! press any key to continue, CTRL+C to abort... ");
  await waitUntilKeypress();
  process.stdout.write("submitting tx... ");
  const { txhash } = await sendTransaction(terra, user, [
    new MsgExecuteContract(
      user.key.accAddress,
      ASTROZAP_ADDR,
      {
        enter: {
          pair: ASTRO_UST_PAIR,
          deposits: [
            {
              info: {
                native: "uusd",
              },
              amount: "1000000",
            },
          ],
          minimum_received: undefined,
        },
      },
      {
        uusd: 1000000,
      },
    ),
  ]);
  console.log("done! txhash:", txhash);
  process.exit(0);
})();
