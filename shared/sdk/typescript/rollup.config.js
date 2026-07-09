import typescript from '@rollup/plugin-typescript';
import { nodeResolve } from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import terser from '@rollup/plugin-terser';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const pkg = require('./package.json');

const banner = `/**
 * ${pkg.name} v${pkg.version}
 * ${pkg.description}
 * @license ${pkg.license}
 */`;

export default [
  // ESM build (for bundlers: Vite, Webpack, etc.)
  {
    input: 'src/index.ts',
    output: {
      file: './dist/index.esm.js',
      format: 'esm',
      banner,
      sourcemap: true,
    },
    plugins: [
      nodeResolve({ browser: true, preferBuiltins: false }),
      commonjs(),
      typescript({
        tsconfig: './tsconfig.json',
        declaration: true,
        declarationDir: './dist',
        rootDir: './src',
      }),
    ],
    external: [
      '@bufbuild/protobuf',
      '@connectrpc/connect',
      '@connectrpc/connect-web',
      '@connectrpc/connect-node',
    ],
  },

  // CJS build (for Node.js)
  {
    input: 'src/index.ts',
    output: {
      file: './dist/index.cjs.js',
      format: 'cjs',
      banner,
      sourcemap: true,
      exports: 'named',
    },
    plugins: [
      nodeResolve({ browser: true, preferBuiltins: false }),
      commonjs(),
      typescript({ tsconfig: './tsconfig.json' }),
    ],
    external: [
      '@bufbuild/protobuf',
      '@connectrpc/connect',
      '@connectrpc/connect-web',
      '@connectrpc/connect-node',
    ],
  },

  // UMD build (for CDN: jsDelivr, unpkg)
  {
    input: 'src/index.ts',
    output: {
      file: './dist/index.umd.js',
      format: 'umd',
      name: 'BennettSDK',
      banner,
      sourcemap: true,
      globals: {
        '@bufbuild/protobuf': 'bufbuildProtobuf',
        '@connectrpc/connect': 'connectrpcConnect',
        '@connectrpc/connect-web': 'connectrpcConnectWeb',
      },
    },
    plugins: [
      nodeResolve({ browser: true, preferBuiltins: false }),
      commonjs(),
      typescript({ tsconfig: './tsconfig.json' }),
      terser({
        format: { comments: /^!/ },
      }),
    ],
  },
];
