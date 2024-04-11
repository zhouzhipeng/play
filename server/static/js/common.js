// 将字符串转换为十六进制
function stringToHex(str) {
    const buffer = new TextEncoder().encode(str);
    return Array.from(buffer).map(b => b.toString(16).padStart(2, '0')).join('');
}

// 将十六进制转换回字符串
function hexToString(hex) {
    const bytes = new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
    return new TextDecoder().decode(bytes);
}