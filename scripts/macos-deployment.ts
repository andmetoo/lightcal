/** Validate both Mach-O slices, independently of the bundle's plist. */
export function validateMacDeploymentTarget(output: string): void {
  const architectures = [...output.matchAll(/\(architecture ([^)]+)\):/g)].map(match => match[1]);
  const commands = [...output.matchAll(/cmd LC_(?:BUILD_VERSION|VERSION_MIN_MACOSX)\b[\s\S]*?\b(?:minos|version)\s+([0-9.]+)/g)];
  if (architectures.length !== 2 || !architectures.includes("x86_64") || !architectures.includes("arm64")
    || commands.length !== 2 || commands.some(command => !/^14\.0(?:\.0)?$/.test(command[1]))) {
    throw new Error("Both universal executable slices must target macOS 14.0.");
  }
}
