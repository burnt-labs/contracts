const fs = require('fs');
const path = require('path');

const testnetSchema = {
  type: 'object',
  required: ['code_id', 'hash', 'network', 'deployed_by', 'deployed_at'],
  properties: {
    code_id: { type: 'string', pattern: '^[0-9]+$' },
    hash: { 
      type: 'string', 
      pattern: '^[a-fA-F0-9]{64}$',
      message: 'Testnet hash must be 64 hex characters long' 
    },
    network: { type: 'string', minLength: 1 },
    deployed_by: { type: 'string', pattern: '^xion[a-z0-9]+$' }, // Basic validation for xion address
    deployed_at: { type: 'string', pattern: '^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}\\.\\d{3}Z$' } // ISO 8601 format
  }
};

const schema = {
  type: 'array',
  items: {
    type: 'object',
    required: ['name', 'description', 'code_id', 'hash', 'release', 'author', 'governance', 'deprecated'],
    properties: {
      name: { type: 'string', minLength: 1 },
      description: { type: 'string' },
      code_id: { type: 'string', pattern: '^[0-9]+$' },
      hash: { 
        type: 'string', 
        pattern: '^[A-F0-9]{64}$',
        message: 'Mainnet hash must be 64 characters long and contain only uppercase hex characters'
      },
      release: {
        type: 'object',
        required: ['url', 'version'],
        properties: {
          url: { 
            type: 'string',
            pattern: '^https://'
          },
          version: { type: 'string', minLength: 1 }
        }
      },
      author: {
        type: 'object',
        required: ['name', 'url'],
        properties: {
          name: { type: 'string', minLength: 1 },
          url: { 
            type: 'string',
            pattern: '^https://'
          }
        }
      },
      governance: { 
        type: 'string',
        pattern: '^(Genesis|[0-9]+)$'
      },
      deprecated: { type: 'boolean' },
      testnet: { ...testnetSchema, optional: true } // Mark testnet as optional
    }
  }
};

function validateJson(data, schema, path = '') {
  if (schema.type === 'array') {
    if (!Array.isArray(data)) {
      throw new Error(`${path} must be an array`);
    }
    
    // Check for duplicate code IDs
    const codeIds = new Set();
    data.forEach((item, index) => {
      if (item.code_id && codeIds.has(item.code_id)) {
        throw new Error(`Duplicate code_id ${item.code_id} found`);
      }
      if (item.code_id) codeIds.add(item.code_id);
      validateJson(item, schema.items, `${path}[${index}]`);
    });

    // Check code_id ordering for all contracts
    for (let i = 1; i < data.length; i++) {
      if (data[i-1].code_id && data[i].code_id) {
        const prevCodeId = parseInt(data[i-1].code_id);
        const currentCodeId = parseInt(data[i].code_id);
        if (currentCodeId < prevCodeId) {
          throw new Error(`Contracts not in code_id order: ${data[i-1].name} (${prevCodeId}) comes before ${data[i].name} (${currentCodeId})`);
        }
      }
    }
    return;
  }

  if (schema.type === 'object') {
    if (typeof data !== 'object' || data === null) {
      throw new Error(`${path} must be an object`);
    }

    // Check required properties
    for (const required of schema.required || []) {
      if (!(required in data)) {
        throw new Error(`${path} missing required property: ${required}`);
      }
    }

    // Validate each property
    for (const [key, value] of Object.entries(data)) {
      const propertySchema = schema.properties[key];
      if (!propertySchema) {
        throw new Error(`${path} has unknown property: ${key}`);
      }
      if (!propertySchema.optional || (key in data)) {
          validateJson(value, propertySchema, `${path}.${key}`);
      }
    }
    return;
  }

  if (schema.type === 'string') {
    if (typeof data !== 'string') {
      throw new Error(`${path} must be a string`);
    }
    if (schema.minLength && data.length < schema.minLength) {
      throw new Error(`${path} must be at least ${schema.minLength} characters`);
    }
    if (schema.pattern) {
      if (path.endsWith('.hash')) {
        console.log(`DEBUG: Validating ${path} with pattern ${schema.pattern}. Value: "${data}" Length: ${data.length}`);
      }
      const regex = new RegExp(schema.pattern);
      if (!regex.test(data)) {
        throw new Error(`${path} ${schema.message || `must match pattern: ${schema.pattern}`}`);
      }
    }
    return;
  }

  if (schema.type === 'boolean') {
    if (typeof data !== 'boolean') {
      throw new Error(`${path} must be a boolean`);
    }
    return;
  }

  throw new Error(`Unknown schema type: ${schema.type}`);
}

try {
  const contractsPath = path.join(__dirname, '../contracts.json');
  const data = JSON.parse(fs.readFileSync(contractsPath, 'utf8'));
  validateJson(data, schema);
  console.log('✅ contracts.json is valid');
  process.exit(0);
} catch (error) {
  if (error.code === 'ENOENT') {
    console.error(`❌ Error: Could not find contracts.json at ${contractsPath}. Make sure the file exists.`);
  } else {
    console.error('❌ Validation Error:', error.message);
  }
  process.exit(1);
}