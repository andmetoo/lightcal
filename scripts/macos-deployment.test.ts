import { expect, test } from "bun:test";
import { validateMacDeploymentTarget } from "./macos-deployment";

const intel = (version: string) => `Lightcal (architecture x86_64):
Load command 9
      cmd LC_VERSION_MIN_MACOSX
  cmdsize 16
  version ${version}
      sdk 27.2`;
const arm = (version: string) => `Lightcal (architecture arm64):
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform MACOS
    minos ${version}
      sdk 27.2
   ntools 1
     tool LD
  version 27106.1`;

test("universal bundle validates executable targets rather than SDK or linker versions", () => {
  expect(() => validateMacDeploymentTarget(`${intel("14.0")}\n${arm("14.0")}`)).not.toThrow();
  expect(() => validateMacDeploymentTarget(`${arm("14.0.0").replace("arm64", "x86_64")}\n${arm("14.0")}`)).not.toThrow();
  expect(() => validateMacDeploymentTarget(`${arm("14.0")}\n${arm("14.0")}`)).toThrow();
  expect(() => validateMacDeploymentTarget(`${intel("10.12")}\n${arm("11.0")}`)).toThrow();
  expect(() => validateMacDeploymentTarget(`${intel("14.0")}\n${arm("11.0")}`)).toThrow();
  expect(() => validateMacDeploymentTarget(arm("14.0"))).toThrow();
  expect(() => validateMacDeploymentTarget("")).toThrow();
});
