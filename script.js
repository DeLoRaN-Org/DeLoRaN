const { exec } = require('child_process')
//287c7cc9
let appkey = 'BBF326BE9AC051453AA616410F110EE7'

//let netSKkey = '589e5156f3b9ae75d665ee75d5fa8359'
//let appSKey = '0b63993a17af02fc75c32a78eb376e05'

let netSKkey = '77495ba01b1e392a537ea623a5e299c3'
let appSKey = 'a4f3ef47ab540cfa0cbedd677d87c2d6'

let p2 = '2005F146AA6C043FE426E3CC81CBEEE4FE427423FB4FC4448AEEBA02DE189430C3'
let p3 = '4014141414a0000000b9ff69468249062c97f2e8fe452bd2d253f3b426f59f81f3c0f90d4f75ca42e9eda7271c80c735a295016e4ba7e418948a6b06f3'
let p4 = '40183710e0000000008ed631667006710f093793db04f7'
let decode = (pl) => {
    let cmd = `echo -n ${pl} | ./ttn-lw-cli lorawan decode --lorawan-version 1.1 --input-format hex --app-key ${appkey} --app-s-key  ${appSKey} --nwk-s-key ${netSKkey}`
    console.log(cmd)
    //exec(cmd, (err, stdout, stderr) => {
    //    console.log(stdout)
    //    console.log(stderr)
    //    //let jval = JSON.parse(stdout);
    //    //console.log(JSON.stringify(jval, null, 2));
    //    //console.log(stderr)
    //})
}

decode(p4)
//decode(p2)