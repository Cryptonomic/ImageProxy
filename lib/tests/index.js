"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
const nft_image_proxy_1 = require("nft-image-proxy");
function test() {
    return __awaiter(this, void 0, void 0, function* () {
        const imageProxyServer = {
            url: "http://localhost:3000",
            version: "1.0.0",
            apikey: "134472c4dd9118dbff1ed4e5fc7f1d056a0d690c9b6cc47c5c2453a011f57127",
        };
        const urls = [
            "https://upload.wikimedia.org/wikipedia/commons/1/1b/GreatBarrierReef-EO.JPG",
            "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg",
        ];
        const fetchResponse = yield nft_image_proxy_1.unsafeFetch(imageProxyServer, "https://upload.wikimedia.org/wikipedia/commons/8/84/Michelangelo%27s_David_2015.jpg", nft_image_proxy_1.ImageProxyDataType.Json);
        const describeResponse = yield nft_image_proxy_1.describe(imageProxyServer, urls);
        const reportResponse = yield nft_image_proxy_1.report(imageProxyServer, urls[0], [nft_image_proxy_1.ModerationLabel.Gambling]);
        const describeReportsResponse = yield nft_image_proxy_1.describeReports(imageProxyServer);
        console.log(fetchResponse);
        console.log(describeResponse);
        console.log(reportResponse);
        console.log(describeReportsResponse);
        console.log(urls[0]);
    });
}
test();
