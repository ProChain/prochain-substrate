// https://eslint.org/docs/user-guide/configuring

module.exports = {
    root: true,

    parserOptions: {
        parser: 'babel-eslint',
        sourceType: 'module'
    },

    env: {
        browser: true,
        node: true
    },

    // https://github.com/standard/standard/blob/master/docs/RULES-en.md
    extends: [
        'plugin:vue/base'
    ],

    // required to lint *.vue files
    plugins: [
        'vue'
    ],

    // add your custom rules here
    'rules': [
        'plugin:vue/base',
        'plugin:vue/essential',
        '@vue/standard'
    ],

    globals: {
        '$': false,
        'jquery': false,
        'ActiveXObject': false,
        'arbor': true,
        'layer': false
    },

    rules: {
        indent: [
            'error',
			'tab'
        ],
        quotes: [
            2,
            'single'
        ],
        'linebreak-style': [
            2,
            'unix'
        ],
        semi: [
            0,
            'always'
        ],
        'no-console': process.env.NODE_ENV === 'production' ? 'error' : 'off',
        'no-unused-vars': [
            1
        ],
        'space-unary-ops': [
            1,
            {
                words: true,
                nonwords: false
            }
        ],
        'brace-style': [
            2,
            '1tbs',
            {
                allowSingleLine: false
            }
        ],
        'comma-spacing': [
            2,
            {
                before: false,
                after: true
            }
        ],
        'comma-style': [
            2,
            'last'
        ],
        'key-spacing': [
            2,
            {
                beforeColon: false,
                afterColon: true
            }
        ],
        'lines-around-comment': [
            2,
            {
                beforeBlockComment: false,
                beforeLineComment: false,
                afterBlockComment: false,
                afterLineComment: false,
                allowBlockStart: true,
                allowObjectStart: true,
                allowArrayStart: true
            }
        ],
        'max-depth': [
            2,
            4
        ],
        'max-len': [
            1,
            1600,
            2
        ],
        'max-nested-callbacks': [
            2,
            3
        ],
        'max-params': [
            2,
            5
        ],
        'max-statements': [
            1,
            80
        ],
        'no-array-constructor': [
            2
        ],
        'no-lonely-if': 2,
        'no-multiple-empty-lines': [
            2,
            {
                max: 3,
                maxEOF: 1
            }
        ],
        'no-nested-ternary': 2,
        'no-spaced-func': 2,
        'no-trailing-spaces': 2,
        'no-unneeded-ternary': 2,
        'object-curly-spacing': [
            2,
            'always',
            {
                objectsInObjects: true,
                arraysInObjects: true
            }
        ],
        'arrow-spacing': 2,
        'block-scoped-var': 2,
        'no-dupe-class-members': 2,
        'object-shorthand': [
            1,
            'always'
        ],
        'array-bracket-spacing': [
            2,
            'never'
        ],
        'operator-linebreak': [
            2,
            'after'
        ],
        'semi-spacing': [
            2,
            {
                before: false,
                after: true
            }
        ],
        'keyword-spacing': [
            'error'
        ],
        'space-before-blocks': 2,
        'block-spacing': [
            2,
            'always'
        ],
        'space-before-function-paren': [
            2,
            'never'
        ],
        'space-in-parens': [
            2,
            'never'
        ],
        'spaced-comment': [
            1,
            'always',
            {
                exceptions: [
                    '-',
                    '*',
                    '+'
                ]
            }
        ],
        'arrow-parens': 0,
        'generator-star-spacing': 0,
        'no-debugger': process.env.NODE_ENV === 'production' ? 'error' : 'off',
		"vue/script-indent": ["error", "tab", {  // script标签缩进设置
			"baseIndent": 1,
			"switchCase": 0,
			"ignores": []
		}]
    },
	overrides: [
	    {
	      "files": ["*.vue"],
	      "rules": {
	        "indent": "off",
	      }
	    }
	]
}
