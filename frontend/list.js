let debugMode = false;

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();

let trainHTMLTable = document.getElementById("train-table");
let trackHTMLTable = document.getElementById("track-table");
let nodeHTMLTable = document.getElementById("node-table");

let derail_img = new Image();
derail_img.id = "derail-img";
derail_img.src = "derail.png";

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
        train.html_row.children[5].children[0].value = current_t;

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
            train.html_row.children[4].innerText = "(" + train.x.toFixed(1).padStart(6, "0")
                + "," + train.y.toFixed(1).padStart(6, "0") + ")";
            train.last_pos_time = time;
        }
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


        while (trainHTMLTable.rows.length > 1) {
            trainHTMLTable.deleteRow(-1);
        }

        while (trackHTMLTable.rows.length > 1) {
            trackHTMLTable.deleteRow(-1);
        }

        while (nodeHTMLTable.rows.length > 1) {
            nodeHTMLTable.deleteRow(-1);
        }

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

                        let new_row = trainlist.get(new_train.id)?.html_row;
                        if (!new_row) {
                            let row_img_src = document.createElement("pre");
                            row_img_src.innerText = new_train.img.src;
                            let row_progress = document.createElement("progress");
                            row_progress.value = 0.0;
                            let button_delete = document.createElement("button");
                            let button_reverse = document.createElement("button");
                            let button_copyid = document.createElement("button");
                            let button_pos = document.createElement("button");

                            button_delete.innerText = "DEL";
                            button_reverse.innerText = "REV";
                            button_copyid.innerText = "ID";
                            button_pos.innerText = "POS";

                            new_row = trainHTMLTable.insertRow(-1);
                            new_row.insertCell(-1); // id
                            new_row.insertCell(-1); // track
                            new_row.insertCell(-1); // direction
                            new_row.insertCell(-1).append(row_img_src); // image src
                            new_row.insertCell(-1); // position
                            new_row.insertCell(-1).append(row_progress); // progress
                            let actions = new_row.insertCell(-1); // actions
                            actions.append(button_delete);
                            actions.append(button_reverse);
                            actions.append(button_copyid);
                            actions.append(button_pos);
                        }

                        let cells = new_row.children;

                        cells[0].innerText = Number(args[0]);
                        cells[1].innerText = new_train.track_id;
                        if (args[4] == "forward") {
                            cells[2].innerText = "=>";
                        } else {
                            cells[2].innerText = "<=";
                        }
                        cells[3].children[0].innerText = new_train.img.src;
                        cells[4].innerText = "idk";
                        cells[5].children[0].value = 0.0;
                        cells[6].children[0].onclick = _ => {
                            socket.send("click\n" + new_train.id + " 1,0,0");
                        };
                        cells[6].children[1].onclick = _ => {
                            socket.send("click\n" + new_train.id + " 0,1,0");
                        };
                        cells[6].children[2].onclick = _ => {
                            navigator.clipboard.writeText(new_train.id);
                        };
                        cells[6].children[3].onclick = _ => {
                            navigator.clipboard.writeText("(" + new_train.x.toFixed(1).padStart(6, "0")
                                + "," + new_train.y.toFixed(1).padStart(6, "0") + ")");
                        };

                        new_train.html_row = new_row;

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

                        let new_row = tracklist.get(new_track.id)?.html_row;
                        if (!new_row) {
                            let train_button = document.createElement("button");
                            train_button.innerText = "TRAIN";

                            new_row = trackHTMLTable.insertRow(-1);
                            new_row.insertCell(-1); // id
                            new_row.insertCell(-1); // start
                            new_row.insertCell(-1); // end
                            new_row.insertCell(-1).append(train_button); // actions
                        }

                        let cells = new_row.children;
                        cells[0].innerText = new_track.id;
                        cells[0].style.backgroundImage = "linear-gradient(to right,white," + new_track.color + ")";
                        cells[1].innerText = "(" + cordlist[0] + "," + cordlist[1] + ")";
                        cells[2].innerText = "(" + cordlist[cordlist.length - 2] + "," + cordlist[cordlist.length - 1] + ")";
                        cells[3].children[0].onclick = () => {
                            ctrl_socket.send("train_new\n" + new_track.id);
                        };

                        new_track.html_row = new_row;

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
                            let x_input = document.createElement("input");
                            x_input.type = "number";
                            let y_input = document.createElement("input");
                            y_input.type = "number";
                            let move_button = document.createElement("button");
                            move_button.innerText = "MOVE";

                            new_row = nodeHTMLTable.insertRow(-1);
                            new_row.insertCell(-1); // id
                            new_row.insertCell(-1).append(x_input); // x position
                            new_row.insertCell(-1).append(y_input); // y position
                            new_row.insertCell(-1).append(move_button); // move_button
                        }

                        let cells = new_row.children;
                        cells[0].innerText = new_node.id;
                        cells[1].children[0].value = new_node.x;
                        cells[2].children[0].value = new_node.y;
                        
                        new_node.x_input = cells[1].children[0];
                        new_node.y_input = cells[2].children[0];
                        
                        cells[3].children[0].onclick = () => {
                            ctrl_socket.send("node_move\n" + new_node.id + " " + new_node.x_input.value + ";" + new_node.y_input.value);
                        };
                        
                        new_node.html_row = new_row;
                        
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

document.getElementById("delete-all").onclick = () => {
    trainlist.forEach(train => {
        socket.send("click\n" + train.id + " 1,0,0");
    })
}
