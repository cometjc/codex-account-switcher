#!/usr/bin/env node

process.env.NODE_ENV = 'development';

require('ts-node/register/transpile-only');

const {execute} = require('@oclif/core');

void execute({development: true, dir: __dirname});
