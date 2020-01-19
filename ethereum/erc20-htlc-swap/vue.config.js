module.exports = {
	productionSourceMap: process.env.NODE_ENV === 'development',
    lintOnSave: true,
    devServer: {
        port: 9000,
		disableHostCheck: true,
	    overlay: {
	      warnings: true,
	      errors: true
	    }
    },
    configureWebpack: {
        externals: {
            'vue': 'Vue',
            'vuex': 'Vuex',
            'vue-router': 'VueRouter',
            'vant': 'vant'
        }
    }
};
