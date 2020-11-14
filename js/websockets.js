
var open_websockets = [];


function start_connect(cb_data_ptr, url_ptr, url_len) {
    let url = UTF8ToString(url_ptr, url_len);

    let ws = new WebSocket(url);

    ws.onopen = function () {
        var inner_id = open_websockets.length;
        open_websockets.push(ws);
        wasm_exports.connected(cb_data_ptr, inner_id);
    }
}

miniquad_add_plugin({
    register_plugin: function (importObject) {
        importObject.env.start_connect = start_connect;
    },
    on_init: function () { }
});
