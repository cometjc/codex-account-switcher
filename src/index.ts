#!/usr/bin/env node
import { run, flush, Errors } from "@oclif/core";
import { routeCliArgv } from "./lib/route-cli-argv";

const routedArgv = routeCliArgv(process.argv.slice(2));

void run(routedArgv)
  .then(() => flush())
  .catch(Errors.handle);
