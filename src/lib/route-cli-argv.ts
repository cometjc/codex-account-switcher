export function routeCliArgv(argv: string[]): string[] {
  return argv.length === 0 ? ["root"] : argv;
}
