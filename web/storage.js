// Lets the game keep each player's progress in the browser's local storage,
// since a web page can't save files. Registered with miniquad as a plugin.
miniquad_add_plugin({
    name: "robot_storage",
    version: 1,
    register_plugin: function (importObject) {
        const bytes = (ptr, len) => new Uint8Array(wasm_memory.buffer, ptr, len);
        const text = (ptr, len) => new TextDecoder().decode(bytes(ptr, len));
        const key = (ptr, len) => "robot_steps/" + text(ptr, len);

        importObject.env.robot_storage_read = function (key_ptr, key_len, buf, cap) {
            let value = null;
            try {
                value = localStorage.getItem(key(key_ptr, key_len));
            } catch (e) {
                // Storage turned off: nothing saved.
            }
            if (value === null) {
                return -1;
            }
            const encoded = new TextEncoder().encode(value);
            if (encoded.length <= cap) {
                bytes(buf, encoded.length).set(encoded);
            }
            return encoded.length;
        };

        importObject.env.robot_storage_write = function (key_ptr, key_len, ptr, len) {
            try {
                localStorage.setItem(key(key_ptr, key_len), text(ptr, len));
            } catch (e) {
                // Storage full or turned off: carry on without saving.
            }
        };
    },
});
