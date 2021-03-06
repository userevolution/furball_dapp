import { Contract, WalletConnection } from "near-api-js";
import React from "react";
import ReactDOM from "react-dom";
import App from "./App";
import { initCeramic } from "./db/ceramic";
import { initIPFS } from './ipfs';
import { initContract } from "./utils";

declare global {
  interface Window {
    nearInitPromise: Promise<any>;
    walletConnection: WalletConnection;
    accountId: any;
    contract: Contract;
    ipfs: any
  }
}

(window as any).nearInitPromise = initContract()
  .then(() => {
    ReactDOM.render(<App />, document.querySelector("#root"));
    console.log(window.contract)
  })
  .then(initIPFS)
  .then(initCeramic)
  .catch(console.error)
