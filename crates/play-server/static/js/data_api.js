/**
 * General Data API v4 Client
 * JavaScript client for interacting with the data API endpoints
 */

class DataAPIClient {
    constructor(baseUrl = '') {
        this.baseUrl = baseUrl || window.location.origin;
        this.apiPath = '/api/v4/data';
    }

    /**
     * Build the full URL for an endpoint
     * @private
     */
    _buildUrl(category, action) {
        return `${this.baseUrl}${this.apiPath}/${category}/${action}`;
    }

    /**
     * Make an HTTP request
     * @private
     */
    async _request(url, options = {}) {
        const defaultOptions = {
            headers: {
                'Content-Type': 'application/json',
            },
        };

        const response = await fetch(url, { ...defaultOptions, ...options });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(`API Error (${response.status}): ${error}`);
        }

        return response.json();
    }

    /**
     * Build query string from parameters
     * @private
     */
    _buildQueryString(params) {
        if (!params || Object.keys(params).length === 0) {
            return '';
        }
        
        const searchParams = new URLSearchParams();
        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== null) {
                searchParams.append(key, value.toString());
            }
        });
        
        const queryString = searchParams.toString();
        return queryString ? `?${queryString}` : '';
    }

    /**
     * Create a new data entry
     * @param {string} category - Category name (2-20 chars, alphanumeric, dash, underscore)
     * @param {Object} data - Data object to store
     * @param {Object} options - Optional parameters
     * @param {boolean} options.unique - If true, ensures only one record exists for this category
     * @returns {Promise<Object>} Created data entry with system fields
     */
    async insert(category, data, options = {}) {
        const url = this._buildUrl(category, 'insert') + this._buildQueryString(options);
        
        return this._request(url, {
            method: 'POST',
            body: JSON.stringify(data),
        });
    }

    /**
     * Retrieve a single data entry
     * @param {string} category - Category name
     * @param {Object} options - Query parameters
     * @param {number} options.id - ID of the entry to retrieve (optional if category has only one record)
     * @param {string} options.select - Comma-separated list of fields to return
     * @param {boolean} options.slim - If true, returns only data object without system fields
     * @returns {Promise<Object>} Data entry
     */
    async get(category, options = {}) {
        const url = this._buildUrl(category, 'get') + this._buildQueryString(options);
        
        return this._request(url, {
            method: 'GET',
        });
    }

    /**
     * Query multiple data entries
     * @param {string} category - Category name
     * @param {Object} options - Query parameters
     * @param {string} options.select - Comma-separated list of fields to return
     * @param {string|number} options.limit - Format: "offset,count" or just count (default: "0,10")
     * @param {string} options.where - SQL-like conditions for filtering
     * @param {string} options.order_by - Field(s) to sort by (default: "id desc")
     * @param {boolean} options.slim - If true, returns only data objects without system fields
     * @param {boolean} options.include_deleted - If true, includes soft-deleted entries
     * @returns {Promise<Array>} Array of data entries
     */
    async query(category, options = {}) {
        const url = this._buildUrl(category, 'query') + this._buildQueryString(options);
        
        return this._request(url, {
            method: 'GET',
        });
    }

    /**
     * Count data entries
     * @param {string} category - Category name
     * @param {Object} options - Query parameters
     * @param {string} options.where - SQL-like conditions for filtering
     * @param {boolean} options.include_deleted - If true, includes soft-deleted entries
     * @returns {Promise<Object>} Object with 'rows' count
     */
    async count(category, options = {}) {
        const url = this._buildUrl(category, 'count') + this._buildQueryString(options);
        
        return this._request(url, {
            method: 'GET',
        });
    }

    /**
     * Update a data entry
     * @param {string} category - Category name
     * @param {number} id - ID of the entry to update
     * @param {Object} data - Fields to update
     * @param {Object} options - Optional parameters
     * @param {boolean} options.override_data - If true, replaces entire data object (default: false, performs JSON patch)
     * @returns {Promise<Object>} Object with 'affected_rows' count
     */
    async update(category, id, data, options = {}) {
        const params = { id, ...options };
        const url = this._buildUrl(category, 'update') + this._buildQueryString(params);
        
        return this._request(url, {
            method: 'POST',
            body: JSON.stringify(data),
        });
    }

    /**
     * Delete data entries
     * @param {string} category - Category name
     * @param {Object} options - Delete parameters
     * @param {number} options.id - ID of the entry to delete (required if delete_all is false)
     * @param {boolean} options.delete_all - If true, deletes all entries in category
     * @param {boolean} options.hard_delete - If true, permanently deletes (default: false, soft delete)
     * @returns {Promise<Object>} Object with 'affected_rows' count
     */
    async delete(category, options = {}) {
        const url = this._buildUrl(category, 'delete') + this._buildQueryString(options);
        
        return this._request(url, {
            method: 'POST',
        });
    }

    /**
     * Helper function to build WHERE clauses for queries
     * @param {Object} conditions - Key-value pairs for conditions
     * @param {string} operator - Logical operator ('AND' or 'OR')
     * @returns {string} WHERE clause string
     */
    buildWhereClause(conditions, operator = 'AND') {
        const clauses = [];
        
        for (const [key, value] of Object.entries(conditions)) {
            if (value === null) {
                clauses.push(`${key} IS NULL`);
            } else if (typeof value === 'string') {
                // Escape single quotes in string values
                const escapedValue = value.replace(/'/g, "''");
                clauses.push(`${key}='${escapedValue}'`);
            } else if (typeof value === 'boolean') {
                clauses.push(`${key}=${value}`);
            } else if (typeof value === 'object' && value.operator && value.value !== undefined) {
                // Support custom operators: {operator: '>', value: 50}
                if (typeof value.value === 'string') {
                    const escapedValue = value.value.replace(/'/g, "''");
                    clauses.push(`${key}${value.operator}'${escapedValue}'`);
                } else {
                    clauses.push(`${key}${value.operator}${value.value}`);
                }
            } else {
                clauses.push(`${key}=${value}`);
            }
        }
        
        return clauses.join(` ${operator} `);
    }

    /**
     * Batch operations helper
     * Execute multiple operations in parallel
     * @param {Array<Object>} operations - Array of operation objects
     * @returns {Promise<Array>} Array of results
     */
    async batch(operations) {
        const promises = operations.map(op => {
            switch (op.type) {
                case 'insert':
                    return this.insert(op.category, op.data, op.options);
                case 'get':
                    return this.get(op.category, op.options);
                case 'query':
                    return this.query(op.category, op.options);
                case 'count':
                    return this.count(op.category, op.options);
                case 'update':
                    return this.update(op.category, op.id, op.data, op.options);
                case 'delete':
                    return this.delete(op.category, op.options);
                default:
                    return Promise.reject(new Error(`Unknown operation type: ${op.type}`));
            }
        });
        
        return Promise.all(promises);
    }
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = DataAPIClient;
}

// Example usage and utility functions
const DataAPI = {
    /**
     * Create a new instance of the API client
     */
    createClient: function(baseUrl) {
        return new DataAPIClient(baseUrl);
    },

    /**
     * Default client instance
     */
    defaultClient: new DataAPIClient(),

    /**
     * Validation utilities
     */
    validation: {
        /**
         * Validate category name
         * @param {string} category - Category name to validate
         * @returns {boolean} True if valid
         */
        isValidCategory: function(category) {
            const pattern = /^[a-zA-Z0-9\-_]{2,20}$/;
            return pattern.test(category);
        },

        /**
         * Validate that field names don't conflict with system fields
         * @param {Object} data - Data object to validate
         * @returns {Array<string>} Array of invalid field names
         */
        getInvalidFields: function(data) {
            const systemFields = ['id', 'cat', 'data', 'is_deleted', 'created', 'updated'];
            return Object.keys(data).filter(key => systemFields.includes(key));
        }
    },

    /**
     * Query builder utilities
     */
    queryBuilder: {
        /**
         * Build a complex WHERE clause with multiple conditions
         * @param {Array<Object>} conditions - Array of condition objects
         * @returns {string} WHERE clause string
         */
        buildComplexWhere: function(conditions) {
            return conditions.map(cond => {
                if (cond.group) {
                    // Handle grouped conditions
                    const groupedWhere = this.buildComplexWhere(cond.conditions);
                    return `(${groupedWhere})`;
                } else {
                    // Handle single condition
                    const { field, operator = '=', value, logicalOperator = 'AND' } = cond;
                    let clause = '';
                    
                    if (value === null) {
                        clause = `${field} IS NULL`;
                    } else if (typeof value === 'string') {
                        const escapedValue = value.replace(/'/g, "''");
                        clause = `${field}${operator}'${escapedValue}'`;
                    } else {
                        clause = `${field}${operator}${value}`;
                    }
                    
                    return cond.logicalOperator ? `${cond.logicalOperator} ${clause}` : clause;
                }
            }).join(' ');
        },

        /**
         * Build pagination parameters
         * @param {number} page - Page number (1-based)
         * @param {number} pageSize - Number of items per page
         * @returns {string} Limit parameter string
         */
        buildPagination: function(page, pageSize) {
            const offset = (page - 1) * pageSize;
            return `${offset},${pageSize}`;
        }
    }
};

// Usage examples (commented out for production)
/*
// Example 1: Basic CRUD operations
async function exampleCRUD() {
    const client = new DataAPIClient();
    
    // Insert
    const newProduct = await client.insert('products', {
        name: 'Product 1',
        price: 99.99,
        active: true
    });
    console.log('Created:', newProduct);
    
    // Get by ID
    const product = await client.get('products', { id: newProduct.id });
    console.log('Retrieved:', product);
    
    // Update
    const updateResult = await client.update('products', newProduct.id, {
        price: 129.99
    });
    console.log('Updated:', updateResult);
    
    // Query with conditions
    const products = await client.query('products', {
        where: 'price>50 AND active=true',
        order_by: 'price asc',
        limit: '0,10'
    });
    console.log('Query results:', products);
    
    // Delete
    const deleteResult = await client.delete('products', { id: newProduct.id });
    console.log('Deleted:', deleteResult);
}

// Example 2: Using helper functions
async function exampleHelpers() {
    const client = DataAPI.defaultClient;
    
    // Build WHERE clause
    const whereClause = client.buildWhereClause({
        status: 'active',
        price: { operator: '>', value: 50 },
        category: 'electronics'
    });
    
    const results = await client.query('products', {
        where: whereClause,
        limit: DataAPI.queryBuilder.buildPagination(2, 20)
    });
    
    console.log('Results:', results);
}

// Example 3: Batch operations
async function exampleBatch() {
    const client = new DataAPIClient();
    
    const operations = [
        { type: 'insert', category: 'products', data: { name: 'Product A', price: 50 } },
        { type: 'insert', category: 'products', data: { name: 'Product B', price: 75 } },
        { type: 'count', category: 'products', options: { where: 'price>25' } },
        { type: 'query', category: 'products', options: { limit: 5 } }
    ];
    
    const results = await client.batch(operations);
    console.log('Batch results:', results);
}

// Example 4: Site settings (unique record per category)
async function exampleUniqueSettings() {
    const client = new DataAPIClient();
    
    // Insert or update site settings
    const settings = await client.insert('site_settings', {
        theme: 'dark',
        language: 'en',
        maintenance_mode: false
    }, { unique: true });
    
    // Get site settings (no ID needed for unique categories)
    const currentSettings = await client.get('site_settings');
    console.log('Settings:', currentSettings);
}
*/