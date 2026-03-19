#!/usr/bin/env node
import { run, flush, Errors } from "@oclif/core";

const argv = process.argv.slice(2);
const routedArgv = argv.length === 0 ? ["root"] : argv;

void run(routedArgv)
  .then(() => flush())
  .catch(Errors.handle);
