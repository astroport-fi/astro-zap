import * as fs from "fs";
import {
  isTxError,
  Msg,
  MsgInstantiateContract,
  MsgStoreCode,
  Wallet,
  LCDClient,
} from "@terra-money/terra.js";

/**
 * Wait for user key press, then continue. Exit if user enters CTRL + C
 *
 * https://stackoverflow.com/questions/19687407/press-any-key-to-continue-in-nodejs
 */
export async function waitUntilKeypress() {
  process.stdin.setRawMode(true);
  return new Promise((resolve) =>
    process.stdin.once("data", (data) => {
      const byteArray = [...data];
      if (byteArray.length > 0 && byteArray[0] === 3) {
        console.log("^C");
        process.exit(1);
      }
      process.stdin.setRawMode(false);
      process.stdout.write("\n");
      resolve(undefined);
    }),
  );
}

/**
 * Send a transaction. Return result if successful, throw error if failed
 *
 * Use uusd for gas payment and mainnet gas prices for default. We could customize it to make the
 * function more flexible, but I'm too lazy for that
 */
export async function sendTransaction(terra: LCDClient, sender: Wallet, msgs: Msg[]) {
  const tx = await sender.createAndSignTx({ msgs });
  const result = await terra.tx.broadcast(tx);
  if (isTxError(result)) {
    throw new Error("transaction failed! raw log: " + result.raw_log);
  }
  return result;
}

/**
 * Upload contract code to LocalTerra, return code ID
 */
export async function storeCode(terra: LCDClient, deployer: Wallet, filepath: string) {
  const code = fs.readFileSync(filepath).toString("base64");
  const result = await sendTransaction(terra, deployer, [
    new MsgStoreCode(deployer.key.accAddress, code),
  ]);
  return parseInt(result.logs[0].eventsByType.store_code.code_id[0]);
}

/**
 * Instantiate a contract from an existing code ID, return contract address
 */
export async function instantiateContract(
  terra: LCDClient,
  deployer: Wallet,
  codeId: number,
  msg: object,
) {
  const result = await sendTransaction(terra, deployer, [
    new MsgInstantiateContract(deployer.key.accAddress, deployer.key.accAddress, codeId, msg),
  ]);
  return result.logs[0].eventsByType.instantiate_contract.contract_address[0];
}
