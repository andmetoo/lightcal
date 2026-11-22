/** CI-only temporary signing material; never writes into the checkout. */
import { chmod, unlink, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { notaryApiKey, notaryFailureMessage, notaryStoreCredentialsCommand } from "./macos-notarization";
const directory = process.env.RUNNER_TEMP;
if (!directory) throw new Error("Run only on an ephemeral GitHub Actions runner");
const keychain = join(directory, "lightcal-signing.keychain-db");
const certificate = join(directory, "lightcal-certificate.p12");
const privateKey = join(directory, "lightcal-sparkle.key");
const notaryKey = join(directory, "lightcal-notary-key.p8");
const run = async (...args: string[]) => {
  const child = Bun.spawn(args, { stdout: "ignore", stderr: "ignore" });
  if (await child.exited !== 0) throw new Error(`${args[0]} failed; verify signing configuration`);
};
if (process.argv.includes("--cleanup")) {
  await run("security", "delete-keychain", keychain).catch(() => {});
  for (const path of [certificate, privateKey, notaryKey]) await unlink(path).catch(() => {});
} else {
  const required = (key: string) => { const value = process.env[key]; if (!value) throw new Error(`Missing ${key}`); return value; };
  // Validate the API key before writing anything, so a bad secret fails early and clearly.
  const apiKey = notaryApiKey(process.env);
  const password = crypto.randomUUID();
  await writeFile(certificate, Buffer.from(required("APPLE_CERTIFICATE_P12"), "base64"), { mode: 0o600 });
  await writeFile(privateKey, required("SPARKLE_PRIVATE_KEY"), { mode: 0o600 });
  await chmod(privateKey, 0o600);
  await run("security", "create-keychain", "-p", password, keychain);
  await run("security", "set-keychain-settings", "-lut", "21600", keychain);
  await run("security", "unlock-keychain", "-p", password, keychain);
  await run("security", "import", certificate, "-P", required("APPLE_CERTIFICATE_PASSWORD"), "-A", "-t", "cert", "-f", "pkcs12", "-k", keychain);
  await run("security", "list-keychains", "-d", "user", "-s", keychain, join(process.env.HOME!, "Library/Keychains/login.keychain-db"));
  await run("security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:", "-s", "-k", password, keychain);
  // The .p8 stays until the always-run cleanup step: the profile may refer to the file
  // rather than hold a copy, and both later notarytool submissions need it.
  await writeFile(notaryKey, apiKey.privateKey, { mode: 0o600 });
  const notary = Bun.spawn(notaryStoreCredentialsCommand(required("APPLE_NOTARY_PROFILE"), notaryKey, apiKey, keychain),
    { stdout: "pipe", stderr: "pipe" });
  const notaryError = (await Promise.all([new Response(notary.stdout).text(), new Response(notary.stderr).text()])).join("\n");
  if (await notary.exited !== 0) throw new Error(notaryFailureMessage(notaryError));
  await Bun.write(Bun.file(process.env.GITHUB_ENV!), `${await Bun.file(process.env.GITHUB_ENV!).text()}SPARKLE_PRIVATE_KEY_FILE=${privateKey}\nAPPLE_SIGNING_KEYCHAIN=${keychain}\n`);
  await unlink(certificate);
}
