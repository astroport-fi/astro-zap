import * as path from "path";
import dotenv from "dotenv";
import yargs from "yargs/yargs";
import { LCDClient, LocalTerra, MnemonicKey, Wallet } from "@terra-money/terra.js";
import { waitUntilKeypress, storeCode, instantiateContract } from "./3_helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      alias: "n",
      type: "string",
      demandOption: true,
    },
    "code-id": {
      alias: "c",
      type: "number",
      default: 0,
      demandOption: false,
    },
  })
  .parseSync();

let terra: LCDClient;
if (argv.network === "mainnet") {
  terra = new LCDClient({
    URL: "https://lcd.terra.dev",
    chainID: "columbus-5",
    gasPrices: "0.15uusd",
    gasAdjustment: 1.4,
  });
} else if (argv.network === "testnet") {
  terra = new LCDClient({
    URL: "https://bombay-lcd.terra.dev",
    chainID: "bombay-12",
    gasPrices: "0.155uusd",
    gasAdjustment: 1.4,
  });
} else if (argv.network === "localterra") {
  terra = new LocalTerra();
} else {
  throw new Error("invalid network: must be ");
}

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

console.log(`network  : ${argv.network}`);
console.log(`codeId   : ${argv["code-id"] == 0 ? "unspecified" : argv["code-id"]}`);
console.log(`deployer : ${deployer.key.accAddress}`);

(async () => {
  let codeId = argv["code-id"];
  if (codeId == 0) {
    process.stdout.write("ready to store code! press any key to continue, CTRL+C to abort... ");
    await waitUntilKeypress();
    process.stdout.write("uploading contract code... ");
    codeId = await storeCode(
      terra,
      deployer,
      path.resolve("../contracts/astrozap/artifacts/astrozap.wasm"),
    );
    console.log(`success! codeId=${codeId}`);
  }

  process.stdout.write("ready to instantiate! press any key to continue, CTRL+C to abort... ");
  await waitUntilKeypress();
  process.stdout.write("instantiating contract... ");
  const contractAddress = await instantiateContract(terra, deployer, codeId, {});
  console.log(`success! address: ${contractAddress}`);

  process.exit(0);
})();
