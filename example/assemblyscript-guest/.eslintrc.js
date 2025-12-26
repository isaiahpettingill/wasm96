module.exports = {
  extends: ['plugin:@shopify/assemblyscript/recommended'],
  rules: {
    // Disable camelcase for external function declarations (C ABI uses snake_case)
    '@shopify/assemblyscript/camelcase': 'off',
    // AssemblyScript has built-in types like Uint8Array
    'no-undef': 'off'
  }
};