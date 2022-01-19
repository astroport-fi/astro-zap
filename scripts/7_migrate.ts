import * as path from "path";
import dotenv from "dotenv";
import { LCDClient, MnemonicKey, Wallet, MsgMigrateCode } from "@terra-money/terra.js";
import { waitUntilKeypress, storeCode, migrateContract } from "./3_helpers";

const ASTROZAP_ADDR = "terra1s7wn47w5kcvn8yefj8y2ffev62a3yureaz4lvc";

const terra = new LCDClient({
  URL: "https://bombay-lcd.terra.dev",
  chainID: "bombay-12",
  gasPrices: "0.155uusd",
  gasAdjustment: 1.4,
});

let deployer: Wallet;
dotenv.config();
if (!process.env.MNEMONIC) {
  throw new Error("mnemonic not provided");
} else {
  deployer = terra.wallet(
    new MnemonicKey({
      mnemonic: process.env.MNEMONIC,
    }),
  );
}

(async () => {
  process.stdout.write("ready to store code! press any key to continue, CTRL+C to abort... ");
  await waitUntilKeypress();
  process.stdout.write("uploading contract code... ");
  const codeId = await storeCode(
    terra,
    deployer,
    path.resolve("../contracts/astrozap/artifacts/astrozap.wasm"),
  );
  console.log(`success! codeId=${codeId}`);

  process.stdout.write("ready to migrate! press any key to continue, CTRL+C to abort... ");
  await waitUntilKeypress();
  process.stdout.write("migrating contract... ");
  const txhash = await migrateContract(terra, deployer, ASTROZAP_ADDR, codeId);
  console.log(`success! txhash: ${txhash}`);

  process.exit(0);
})();
