#!/usr/bin/env node

process.env.NODE_ENV = 'development';

require('ts-node/register/transpile-only');

const {execute} = require('@oclif/core');
const {routeCliArgv} = require('../src/lib/route-cli-argv');

void execute({
  development: true,
  dir: __dirname,
  args: routeCliArgv(process.argv.slice(2)),
});
