let debugMode = false;

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();

let derail_img = new Image();
derail_img.id = "derail-img";
derail_img.src = "derail.png";
function newtrainfunc(recid){
    ctrl_socket.send("train_new\n" + recid + " " + (Math.random() * 200 + 400));
}
function rev(trainid){
    console.log("rev trid")
    socket.send("click\n" + trainid + " 0,1,0");
}
function redraw(time) {
    trainlist.forEach((train, id) => {
        if (Number.isNaN(train.movement_start)) {
            if (train.direction == 1) {
                train.movement_start = time - Number(train.start_t) * Number(train.duration);
            } else {
                train.movement_start = time - (1 - Number(train.start_t)) * Number(train.duration);
            }
        }

        let cordlist = tracklist.get(train.track_id).cordlist;
        let current_t = (time - train.movement_start) / train.duration;
        if (train.direction == -1) {
            current_t = 1 - current_t;
        }
        train.current_t = current_t;
        let recordindex = grid2.get(id,true);
        grid2.records[recordindex].progress=current_t;
        

        // if (current_t > 1.1 || current_t < -0.1) {
        //     train.html_row.remove();
        //     trainlist.delete(train.id);
        //     return;
        // }

        let point = bezierPoint(cordlist, current_t);
        let x_pos = point.x;
        let y_pos = point.y;
        train.x = x_pos;
        train.y = y_pos;
        if (Number.isNaN(train.last_pos_time) || time - train.last_pos_time > 200) {
            grid2.records[recordindex].position="(" + train.x.toFixed(1).padStart(6, "0")
                + "," + train.y.toFixed(1).padStart(6, "0") + ")";
            train.last_pos_time = time;
        }
        grid2.update();
        // let dresult = bezierDerivative(cordlist, current_t);
        // let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
    });

    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
let socket = null;
let ctrl_socket = null;
function startSocket() {
    socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
    ctrl_socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws-ctrl");


    socket.onerror = _ => {
        document.getElementById("status-container").innerText = "";
        document.getElementById("status-container").append(derail_img);
        window.setTimeout(startSocket, 500);
    };

    socket.onopen = (event) => {
        trainlist = new Map();
        tracklist = new Map();
        nodelist = new Map();
        explosionSerial = 0;
        explosionList = new Map();




        derail_img.remove();
        document.getElementById("status-container").innerText = "Connected! " + new Date();;

        socket.onmessage = (msg) => {
            if (debugMode) {
                console.log(msg);
            }
            let msg_split = msg.data.split("\n");
            // * ! BLIND start here
            let row_count = 1;
            //prase here
            switch (msg_split[0]) {
                case "train":
                    {
                        args = msg_split[1].split(" ");
                        let new_train = {};
                        new_train.id = Number(args[0]);
                        new_train.track_id = Number(args[1]);
                        new_train.start_t = Number(args[2]);
                        new_train.duration = Number(args[3]);
                        if (args[4] == "forward") {
                            new_train.direction = 1;
                        } else {
                            new_train.direction = -1;
                        }
                        new_train.img = new Image();
                        new_train.img.src = msg_split[2];
                        new_train.movement_start = NaN;
                        new_train.last_pos_time = NaN;
                        new_train.x = NaN;
                        new_train.y = NaN;

                        let direction;
                        if (args[4] == "forward") {
                            direction = "=>";
                        } else {
                            direction = "<=";
                        }
                        if(grid2.get(new_train.id)==null){
                            grid2.add({recid:new_train.id,
                                track:new_train.track_id,
                                direction:direction,
                                image:new_train.img.src,
                                position:"idk",
                                progress:0.0});
                        }else{
                            let recordindex = grid2.get(new_train.id,true);
                            grid2.records[recordindex].track=new_train.track_id;
                            grid2.records[recordindex].direction=direction;
                            grid2.records[recordindex].image=new_train.img.src;
                            grid2.update();
                            //not sure if rest need update ort not
                        }

                        trainlist.set(Number(args[0]), new_train);
                    }
                    break;
                case "track":
                    for (i = 2; i < msg_split.length; i++) {
                        args = msg_split[i].split(" ");
                        let new_track = {};
                        let cordlist = args[1].split(";").map(x => Number(x));
                        cordlist.shift();
                        new_track.id = Number(args[0]);
                        new_track.cordlist = cordlist
                        new_track.color = args[2];
                        new_track.thickness = Number(args[3]);
                        new_track.length = bezierRoughLength(cordlist);

                        if(grid1.get(new_track.id)==null){
                            grid1.add({recid:new_track.id,start:"(" + cordlist[0] + "," + cordlist[1] + ")",end:"(" + cordlist[cordlist.length - 2] + "," + cordlist[cordlist.length - 1] + ")",color:new_track.color});
                        }else{
                            let recordindex = grid1.get(new_track.id,true);
                            grid1.records[recordindex].start="(" + cordlist[0] + "," + cordlist[1] + ")";
                            grid1.records[recordindex].end="(" + cordlist[cordlist.length - 2] + "," + cordlist[cordlist.length - 1] + ")";
                            grid1.records[recordindex].color=new_track.color;
                            grid1.update();
                        }
                        tracklist.set(Number(args[0]), new_track);
                    }
                    break;
                case "node":
                    {
                        args = msg_split[1].split(" ");
                        let new_node = {};
                        new_node.id = Number(args[0]);
                        new_node.x = Number(args[1].split(";")[0]);
                        new_node.y = Number(args[1].split(";")[1]);

                        let new_row = nodelist.get(new_node.id)?.html_row;
                        if (!new_row) {
                            //! Do not support multi control
                            //if not exist
                            if(grid.get(new_node.id)==null){
                                grid.add({recid:new_node.id,PositionX:new_node.x,PositionY:new_node.y,nodetype:"Random"});
                            }else{
                                let recordindex = grid.get(new_node.id,true);
                                grid.records[recordindex].PositionX=new_node.x;
                                grid.records[recordindex].PositionY=new_node.y;
                                grid.records[recordindex].nodetype="Random";
                                grid.update();
                            }
                            
                        }


                        nodelist.set(new_node.id, new_node);
                    }
                    break;
                case "remove":
                    args = msg_split[1].split(" ");
                    let new_explosion = {};

                    let removed_id = Number(args[0]);
                    let removal_type = args[1][0];
                    let removed_train = trainlist.get(removed_id);

                    removed_train?.html_row?.remove();
                    trainlist.delete(removed_id);

                    new_explosion.start = NaN;
                    new_explosion.x = removed_train.x;
                    new_explosion.y = removed_train.y;
                    let cordlist = tracklist.get(removed_train.track_id).cordlist;
                    new_explosion.dxdy = bezierDerivative(cordlist, removed_train.current_t);
                    new_explosion.dx = bezierDerivative(cordlist, removed_train.current_t).dx;
                    new_explosion.dy = bezierDerivative(cordlist, removed_train.current_t).dy;
                    new_explosion.cordlist = cordlist;
                    new_explosion.type = removal_type; // e s d v t
                    new_explosion.train = removed_train;

                    // silent explosion require no further animation
                    if (removal_type != "s") {
                        explosionList.set(explosionSerial, new_explosion);
                    }
                    explosionSerial++;

                    break;
            }
        };

        socket.onclose = _ => {
            document.getElementById("status-container").innerText = "";
            document.getElementById("status-container").append(derail_img);
            window.setTimeout(startSocket, 500);
        };
    };
}

startSocket();

// document.getElementById("delete-all").onclick = () => {//TODO: remove for current version
//     trainlist.forEach(train => {
//         socket.send("click\n" + train.id + " 1,0,0");
//     })
// }

let track_select = document.getElementById("track-select");
track_select.addEventListener("click", _ => {
    fetch("/available-tracks").then(response => { return response.json(); })
        .then(list => {
            let track_select = document.getElementById("track-select");
            let current_value = track_select.value;
            track_select.innerHTML = "<option value=\"\">--DEFAULT TRACK--</option>";
            for (track_name of list) {
                let option = document.createElement("option");
                option.innerHTML = track_name;
                option.value = track_name;
                track_select.append(option);
            }
            track_select.value = current_value;
        });
});
