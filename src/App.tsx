import React from "react";
import "react-bootstrap";
import { BrowserRouter as Router, Route, Switch } from "react-router-dom";
import "regenerator-runtime/runtime";
import { Header } from "./components/Header";
import getConfig from "./config/config";
import "./global.css";
import { Artist } from "./pages/Artist";
import { Artwork } from "./pages/Artwork";
import { Profile } from "./pages/Profile";
import { Lookup } from "./pages/Lookup";
import { login } from "./utils";

const { networkId } = getConfig(process.env.NODE_ENV || "development");
export default function App() {

  // if not signed in, return early with sign-in prompt
  if (!window.walletConnection.isSignedIn()) {
    return (
      <main>
        <h1>Welcome to FurBall</h1>
        <p>
          To add your images you need to sign in. The button below will sign you
          in using NEAR Wallet.
        </p>
        <p>
          By default, when your app runs in "development" mode, it connects to a
          test network ("testnet") wallet. This works just like the main network
          ("mainnet") wallet, but the NEAR Tokens on testnet aren't convertible
          to other currencies – they're just for testing!
        </p>
        <p>Go ahead and click the button below to try it out:</p>
        <p style={{ textAlign: "center", marginTop: "2.5em" }}>
          <button onClick={login}>Sign in</button>
        </p>
      </main>
    );
  } else {
    return (
      // use React Fragment, <>, to avoid wrapping elements in unnecessary divs
      <Router>
        <Header />
        <Switch>
          <Route path="/user:accountID">
            <Artist />
          </Route>
          <Route path="/artwork:artCID">
            <Artwork />
          </Route>
          <Route path="/lookup">
            <Lookup />
          </Route>
          <Route path="/">
            <Profile />
          </Route>
        </Switch>
      </Router>
    );
  }
}
