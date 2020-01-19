import { localize } from 'vee-validate'
import { required, min_value } from 'vee-validate/dist/rules'
import { extend } from 'vee-validate'
import zh_CN from 'vee-validate/dist/locale/zh_CN.json'

localize({
	zh_CN
})

localize('zh_CN')

extend('required', required)

extend('min_value', {
	...min_value,
	validate(value, args) {
		return value >= args.number
	},
	message: '最少{number}个PRA',
	params: ['number']
});

// extend('positive', value => {
//     if (value >= 0) {
//         return true
//     }
//     return 'This {_field_} field must be a positive number'
// })
