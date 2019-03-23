const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const HtmlWebpackPlugin = require('html-webpack-plugin');
const HtmlWebpackInlineSourcePlugin = require('html-webpack-inline-source-plugin');

module.exports = {
    entry: ['./src/main.js'],
    resolve: {
        alias: {}
    },
    module: {
        rules: [{
            test: /\.css$/,
            use: [
                MiniCssExtractPlugin.loader, 'css-loader',
            ]
        }, {
            test: /\.html$/,
            use: 'html-loader'
        }]
    },
    plugins: [
        new MiniCssExtractPlugin({
            filename: "index.css",
        }),
        new HtmlWebpackPlugin({
            inlineSource: '.(js|css)$',
            template: 'src/index.html'
        }),
        new HtmlWebpackInlineSourcePlugin()
    ]
};
