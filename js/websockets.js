
function string_to_rust(s) {
    var len = s.length;
    var ptr = wasm_exports.allocate_vec_u8(len);
    var heap = new Uint8Array(wasm_memory.buffer, ptr, len);
    stringToUTF8(s, heap, 0, len);
    return {
        ptr: ptr,
        len: len,
    }
}


var websockets = {
    open: [],

    start_connect: function start_connect(cb_data_ptr, url_ptr, url_len) {
        let url = UTF8ToString(url_ptr, url_len);

        let ws = new WebSocket(url);

        ws.onopen = function () {
            var inner_id = websockets.open.length;
            websockets.open.push(ws);
            var cb_data_ptr2 = wasm_exports.on_open(cb_data_ptr, inner_id);
            console.log("Setting up onmessage")
            ws.onmessage = function(event) {
                console.log("onmessge", event)
                var msg = string_to_rust(event.data);
                wasm_exports.on_message(cb_data_ptr2, msg.ptr, msg.len);
            }
            ws.onclose = function(event) {
                var reason = string_to_rust(event.reason);
                wasm_exports.on_close(cb_data_ptr2, event.code, reason.ptr, reason.len, event.wasClean);
            }
            ws.onerror = function () {
                wasm_exports.on_error(cb_data_ptr2);
            }
        }

        ws.onerror = function () {
            wasm_exports.on_connection_failed(cb_data_ptr);
        }
    },

    send: function send(inner_id, msg_ptr, msg_len) {
        let ws = websockets.open[inner_id];
        let msg = UTF8ToString(msg_ptr, msg_len);
        console.log("Sent", msg)
        ws.send(msg);
    }
}


miniquad_add_plugin({
    register_plugin: function (importObject) {
        importObject.env.websocket_start_connect = websockets.start_connect;
        importObject.env.websocket_send = websockets.send;
    },
    on_init: function () { }
});
