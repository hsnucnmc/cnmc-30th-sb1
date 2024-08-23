const train_width = 350 / 4; // TODO: adjust with image size
const train_height = 263 / 4;

let relative_x = Number(document.cookie.split("; ").find(row => row.startsWith("relative_x="))?.split("=")[1]);
let relative_y = Number(document.cookie.split("; ").find(row => row.startsWith("relative_y="))?.split("=")[1]);

{
    const urlParams = new URLSearchParams(window.location.search);
    let param_x = urlParams.get('x');
    let param_changed = false;
    if (param_x != undefined && !Number.isNaN(param_x)) {
        relative_x = param_x;
        urlParams.delete('x');
        param_changed = true;
    }

    let param_y = urlParams.get('y');
    if (param_y != undefined && !Number.isNaN(param_y)) {
        relative_y = param_y;
        urlParams.delete('y');
        param_changed = true;
    }

    let ask_attempt = 0;
    while (Number.isNaN(relative_x) || relative_x < -4000 || 4000 < relative_x) {
        relative_x = Number(window.prompt("Relative x?", "0"));
        ask_attempt++;
        if (ask_attempt > 10) {
            relative_x = 0;
        }
    }

    ask_attempt = 0;
    while (Number.isNaN(relative_y) || relative_y < -2000 || 2000 < relative_y) {
        relative_y = Number(window.prompt("Relative y?", "0"));
        ask_attempt++;
        if (ask_attempt > 10) {
            relative_y = 0;
        }
    }

    document.cookie = "relative_x=" + relative_x;
    document.cookie = "relative_y=" + relative_y;
    if (param_changed) {
        window.history.replaceState(null, "", window.location.pathname);
    }
}


let debugMode = false;
let dragMode = true;

// TODO: ask for x y boundaries and scale track base on view port size
document.cookie = "relative_x=" + relative_x;
document.cookie = "relative_y=" + relative_y;

const main_canvas = document.getElementById("main-canvas");

function resizeCanvas() {
    main_canvas.width = window.innerWidth;
    main_canvas.height = window.innerHeight;
}

window.addEventListener("resize", resizeCanvas);
resizeCanvas();

let derail_img = new Image();
derail_img.id = "derail-img";
derail_img.src = "derail.png";

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();
let explosionSerial = 0;
let explosionList = new Map();
let trainposition = [];

function redraw(time) {
    // /**
    // * @param trainlist a list of param including (trainid, trackid)
    // * @param st starting time
    // * @param duration uh the duration?
    // */

    let main_canvas = document.getElementById("main-canvas");
    let main_context = main_canvas.getContext("2d");

    main_context.clearRect(0, 0, main_canvas.width, main_canvas.height);
    main_context.save();
    main_context.translate(-relative_x, -relative_y);

    nodelist.forEach(node => {
        if (node.clickable) {
            main_context.fillStyle = "#DD8";
        } else {
            main_context.fillStyle = "#BBB";
        }
        main_context.beginPath();
        main_context.arc(node.x, node.y, 40, 0, 2 * Math.PI);
        main_context.fill();

        switch (node.type) {
            case "reverse": {
                main_context.fillStyle = "#666";
                main_context.beginPath();
                main_context.arc(node.x, node.y, 25, 0, 2 * Math.PI);
                main_context.fill();
            }
                break;

            case "derail": {
                main_context.fillStyle = "#B66";
                main_context.beginPath();
                main_context.arc(node.x, node.y, 40, 0, 2 * Math.PI);
                main_context.fill();
            }
                break;

            case "configurable": {
                main_context.fillStyle = "#66B";
                main_context.beginPath();
                main_context.arc(node.x, node.y, 25, 0, 2 * Math.PI);
                main_context.fill();
            }
                break;
        }
    });

    tracklist.forEach(track => {
        drawTrack(main_context, track);
        track.control_points.forEach(point => {
            switch (point.type) {
                case "none":
                    main_context.fillStyle = "#666666";
                    break;
                case "b2":
                    main_context.fillStyle = "#AA6666";
                    break;
                case "b31":
                    main_context.fillStyle = "#A6F";
                    break;
                case "b32":
                    main_context.fillStyle = "#A6C";
                    break;
            }

            main_context.beginPath();
            main_context.arc(point.x, point.y, 25, 0, 2 * Math.PI);
            main_context.fill();
        }
        )
    });

    explosionList.forEach((explosion, explosion_id) => {
        if (Number.isNaN(explosion.start)) {
            explosion.start = time;
        }

        switch (explosion.type) {
            case "e": // explosion
                if (time - explosion.start > 5000) {
                    explosionList.delete(explosion_id);
                }
                break;
            case "d": // derail
                {
                    let train = explosion.train;
                    if (Number.isNaN(train.movement_start)) {
                        if (train.direction == 1) {
                            train.movement_start = time - Number(train.start_t) * Number(train.duration);
                        } else {
                            train.movement_start = time - (1 - Number(train.start_t)) * Number(train.duration);
                        }
                    }

                    if (tracklist.has(train.track_id)) {
                        let cordlist = tracklist.get(train.track_id).cordlist;
                        let current_t = (time - train.movement_start) / train.duration;
                        if (train.direction == -1) {
                            current_t = 1 - current_t;
                        }
                        train.current_t = current_t;
                        if (current_t > 1.5 || current_t < -0.5) {
                            explosionList.delete(explosion_id);
                            return;
                        }
                        if (current_t > 1) {
                            current_t = (Math.log(5 * current_t - 4)) / 5 + 1;
                        }

                        if (current_t < 0) {
                            current_t = -(Math.log(1 - 5 * current_t)) / 5;
                        }

                        let point = bezierPoint(cordlist, current_t);
                        let x_pos = point.x;
                        let y_pos = point.y;
                        train.x = x_pos;
                        train.y = y_pos;
                        let dresult = bezierDerivative(cordlist, current_t);
                        let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI + current_t * (-22.5) * (train.direction + 1) + (1 - current_t) * (-22.5) * (train.direction - 1);
                        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, train.img);
                    }
                }
                break;
            case "v": // vibration
                {
                    if (time - explosion.start > 1000) {
                        explosionList.delete(explosion_id);
                    }
                    let x = (time - explosion.start) / 333;
                    let vibration_degree = 90 * Math.sin(20 / (x + 6 - 9.25) + 1) * (Math.pow(x, 0.5) / 3);
                    let deg = Math.atan2(explosion.dy, explosion.dx) * 180 / Math.PI;
                    let un_normalized = JSON.parse(JSON.stringify(explosion.dxdy));
                    un_normalized.dx *= explosion.train.direction;
                    un_normalized.dy *= explosion.train.direction;
                    let speed = Math.sqrt(Math.pow(un_normalized.dx, 2) + Math.pow(un_normalized.dy, 2));
                    let dx = un_normalized.dx / speed;
                    let dy = un_normalized.dy / speed;

                    let x_pos = explosion.x + dx * x * 50;
                    let y_pos = explosion.y + dy * x * 50;

                    drawRotatedImg(main_context, x_pos, y_pos, deg + vibration_degree, x_pos - train_width / 2, y_pos - train_height, explosion.train.img);
                }
                break;
            case "t": // take off
                {
                    let fly_distance = 250 * (Math.pow(Math.E, (time - explosion.start) / 600.0) - 1);
                    console.log(fly_distance);
                    if (fly_distance > 2000) {
                        explosionList.delete(explosion_id);
                    }

                    let deg = Math.atan2(explosion.dy, explosion.dx) * 180 / Math.PI;
                    let un_normalized = explosion.dxdy;
                    let speed = Math.sqrt(Math.pow(un_normalized.dx, 2) + Math.pow(un_normalized.dy, 2));
                    let dx = un_normalized.dx / speed;
                    let dy = un_normalized.dy / speed;

                    let x_pos = explosion.x + dy * fly_distance;
                    let y_pos = explosion.y - dx * fly_distance;

                    drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, explosion.train.img);
                }
                break;
        }
    });

    trainposition = [];
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
        if (current_t > 1.1 || current_t < -0.1)
            return;
        let point = bezierPoint(cordlist, current_t);
        let x_pos = point.x;
        let y_pos = point.y;
        let trainpositionitem = {};
        train.x = x_pos;
        train.y = y_pos;
        trainpositionitem.id = id;
        trainpositionitem.x = x_pos;
        trainpositionitem.y = y_pos;
        trainposition.push(trainpositionitem);
        // now take the derivative of track at the position of the train
        let dresult = bezierDerivative(cordlist, current_t);
        let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, train.img);
    });


    main_context.fillStyle = "#888";
    main_context.font = "40px monospace";
    nodelist.forEach(node => {
        main_context.fillText(node.id.toString().padStart(3, "0"), node.x - 20, node.y - 25);
    });

    main_context.restore();
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
        main_canvas.hidden = true;
        document.getElementById("main-canvas").parentElement.append(derail_img);
        window.setTimeout(startSocket, 500);
    };

    socket.onopen = _ => {
        trainlist = new Map();
        tracklist = new Map();
        nodelist = new Map();
        explosionSerial = 0;
        explosionList = new Map();
        trainposition = [];

        main_canvas.hidden = false;
        derail_img.remove();
        // socket.send("position\n" + left_bound + " " + right_bound);
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
                    // code block
                    args = msg_split[1].split(" ");
                    let new_train = {};
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
                    new_train.x = NaN;
                    new_train.y = NaN;

                    trainlist.set(Number(args[0]), new_train);
                    break;
                case "track":
                    tracklist.clear();
                    for (i = 2; i < msg_split.length; i++) {
                        args = msg_split[i].split(" ");
                        let track = {};
                        let cordlist = args[1].split(";").map(x => Number(x));
                        cordlist.shift();
                        track.cordlist = cordlist;
                        track.control_points = [];
                        if (track.cordlist.length == 4) {
                            track.control_points.push({ x: (cordlist[0] + cordlist[2]) / 2, y: (cordlist[1] + cordlist[3]) / 2, track: track, type: "none" });
                        }
                        if (track.cordlist.length == 6) {
                            track.control_points.push({ x: cordlist[2], y: cordlist[3], track: track, type: "b2" });
                            // track.control_points.push(bezierPoint(cordlist, 0.5));
                            // track.control_points[1].track = track;
                        }
                        if (track.cordlist.length == 8) {
                            track.control_points.push({ x: cordlist[2], y: cordlist[3], track: track, type: "b31" });
                            track.control_points.push({ x: cordlist[4], y: cordlist[5], track: track, type: "b32" });
                            track.control_points.push(bezierPoint(cordlist, 0.5));
                            track.control_points[2].track = track;
                            track.control_points[2].type = "none";
                        }

                        track.color = args[2];
                        track.thickness = Number(args[3]);
                        track.length = bezierRoughLength(cordlist);
                        track.id = Number(args[0]);

                        tracklist.set(Number(args[0]), track);
                    }

                    if (selected_cp && !tracklist.has(selected_cp.track.id)) {
                        console.log("track deleted?");
                        selected_cp = null;
                    }
                    updateSelectedAsCP();

                    break;
                case "node":
                    args = msg_split[1].split(" ");
                    let new_node = {};
                    new_node.id = Number(args[0]);
                    new_node.x = Number(args[1].split(";")[0]);
                    new_node.y = Number(args[1].split(";")[1]);
                    new_node.type = undefined;
                    new_node.routing = undefined;
                    new_node.clickable = false;
                    nodelist.set(new_node.id, new_node);

                    fetch("/nodes/" + new_node.id).then(response => {
                        return response.json();
                    }).then(type => {
                        new_node.type = type;
                        if (type == "configurable") {
                            fetch("/nodes/" + new_node.id + "/routing").then(response => {
                                return response.json();
                            }).then(routing => {
                                new_node.routing = routing;
                                fetch("/nodes/" + new_node.id + "/state").then(response => {
                                    return response.json();
                                }).then(state_id => {
                                    if (new_node.routing.states[state_id].after_click != "Nothing") {
                                        new_node.clickable = true;
                                        // console.log("Node#" + new_node.id + " is clickable!");
                                    };
                                });
                            });
                        }
                    });

                    if (selected_node)
                        updateSelectedAsNode();
                    break;
                case "nuke":
                    nodelist.delete(Number(msg_split[1]));
                    if (selected_node.id == Number(msg_split[1])) {
                        selected_node = null;
                        updateSelectedAsNode();
                    }
                    break;
                case "remove":
                    args = msg_split[1].split(" ");
                    let new_explosion = {};

                    let removed_id = Number(args[0]);
                    let removal_type = args[1][0];
                    let removed_train = trainlist.get(removed_id);

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
                        explosionSerial++;
                    }
                    break;
            }
        };

        socket.onclose = _ => {
            main_canvas.hidden = true;
            document.getElementById("main-canvas").parentElement.append(derail_img);
            window.setTimeout(startSocket, 500);
        };
    };
}

startSocket();

let selected_node = null;
let selected_train = null;
let selected_cp = null;

function updateSelectedAsNode() {
    let selected_display = document.getElementById("selected-display");
    if (selected_node === null) {
        selected_display.innerHTML = "Selecetd: None";
        return;
    }

    selected_display.innerHTML = "Selected: Node#" + selected_node.id + "<br/>"
        + "Node Type: ??????<br/>";
    fetch("/nodes/" + selected_node.id).then(response => {
        return response.json();
    }).then(type => {
        selected_display.innerHTML = "Selected: Node#" + selected_node.id + "<br/>"
            + "Node Type: " + type + "<br/>";
        let delete_button = document.createElement("button");
        delete_button.innerText = "Delete Node";
        delete_button.onclick = _ => {
            ctrl_socket.send("node_delete\n" + selected_node.id);
        }
        selected_display.append(delete_button);
    });

    let delete_button = document.createElement("button");
    delete_button.innerText = "Delete Node";
    delete_button.onclick = _ => {
        ctrl_socket.send("node_delete\n" + selected_node.id);
        selected_node = null;
    }
    selected_display.append(delete_button);
}

function updateSelectedAsCP() {
    let selected_display = document.getElementById("selected-display");
    if (selected_cp === null) {
        selected_display.innerHTML = "Selecetd: None";
        return;
    }

    let track_type = "Straight";
    if (selected_cp.track.cordlist.length == 6) {
        track_type = "Quadratic Bezier";
    }
    if (selected_cp.track.cordlist.length == 8) {
        track_type = "Cubic Bezier";
    }

    selected_display.innerHTML = "Selected: Track#" + selected_cp.track.id + "<br/>"
        + "Track Type: " + track_type + "<br/>";

    let delete_button = document.createElement("button");
    delete_button.innerText = "Delete Track";
    delete_button.onclick = _ => {
        ctrl_socket.send("track_delete\n" + selected_cp.track.id);
        selected_cp = null;
    }

    let setBezier2 = document.createElement("button");
    setBezier2.innerText = "Set to Quadratic Bezier";
    setBezier2.onclick = _ => {
        let mid = bezierPoint(selected_cp.track.cordlist, 0.5);
        ctrl_socket.send("track_adjust\n" + selected_cp.track.id + " " + mid.x + ";" + mid.y);
        selected_cp = null;
    }

    let setBezier3 = document.createElement("button");
    setBezier3.innerText = "Set to Cubic Bezier";
    setBezier3.onclick = _ => {
        let mid1 = bezierPoint(selected_cp.track.cordlist, 0.3);
        let mid2 = bezierPoint(selected_cp.track.cordlist, 0.7);
        ctrl_socket.send("track_adjust\n" + selected_cp.track.id
            + " " + mid1.x + ";" + mid1.y
            + "," + mid2.x + ";" + mid2.y);
        selected_cp = null;
    }

    let addTrain = document.createElement("button");
    addTrain.innerText = "Add Train";
    addTrain.onclick = _ => {
        ctrl_socket.send("train_new\n" + selected_cp.track.id + " " + (Math.random() * 200 + 400));
    }

    selected_display.append(delete_button);
    selected_display.append(document.createElement("br"));
    selected_display.append(setBezier2);
    selected_display.append(document.createElement("br"));
    selected_display.append(setBezier3);
    selected_display.append(document.createElement("br"));
    selected_display.append(addTrain);
}

main_canvas.addEventListener("mousedown", event => {
    if (event.button === 2) {
        let x = event.offsetX / 0.7 + relative_x;
        let y = event.offsetY / 0.7 + relative_y;
        ctrl_socket.send("node_new\n" + x + ";" + y + " random");
        return;
    }
});


// update click on demand
main_canvas.addEventListener("click", function (event) {
    let x = event.offsetX / 0.7 + relative_x;
    let y = event.offsetY / 0.7 + relative_y;
    let mousePos = { x: x, y: y };

    let cp_clicked = false;
    tracklist.forEach(track => {
        track.control_points.forEach(point => {
            const CONTROL_POINT_R = 25 / 0.7;
            clickr = Math.sqrt(Math.pow(mousePos.x - point.x, 2) + Math.pow(mousePos.y - point.y, 2));
            if (clickr <= CONTROL_POINT_R) {
                if (event.shiftKey) {
                    if (selected_node) {
                        // ctrl_socket.send("track_new\n" + selected_node.id + " " + node.id + " #6CF");
                        cp_clicked = true;
                    }
                } else {
                    selected_cp = point;
                    cp_clicked = true;
                }
            }
        });
    });
    
    if (cp_clicked) {
        selected_train = null;
        selected_node = null;
        updateSelectedAsCP();
        return;
    }
    
    let node_r = 40 / 0.7;
    let node_clicked = false;
    nodelist.forEach(node => {
        clickr = Math.sqrt(Math.pow(mousePos.x - node.x, 2) + Math.pow(mousePos.y - node.y, 2));
        if (clickr <= node_r) {
            // socket.send("switch\n" + node.id + " " + Number(event.ctrlKey) + "," + Number(event.shiftKey) + "," + Number(event.altKey));
            if (event.shiftKey) {
                if (selected_node) {
                    ctrl_socket.send("track_new\n" + selected_node.id + " " + node.id + " #6CF");
                    node_clicked = true;
                }
            } else {
                selected_node = node;
                node_clicked = true;
            }
        }
    });

    if (node_clicked) {
        selected_train = null;
        selected_cp = null;
        updateSelectedAsNode();
        return;
    }

    let train_clicked = false;
    let train_r = Math.sqrt(Math.pow(train_width / 2, 2) + Math.pow(train_height / 2, 2)) / 0.7;
    // time complexity (o(n))
    trainposition.forEach(pos => {
        clickr = Math.sqrt(Math.pow(mousePos.x - pos.x, 2) + Math.pow(mousePos.y - pos.y, 2));
        if (clickr <= train_r) {
            socket.send("click\n" + pos.id + " " + Number(event.ctrlKey) + "," + Number(event.shiftKey) + "," + Number(event.altKey));
            train_clicked = true;
            selected_train = pos.id;
        }
    });

    if (train_clicked) {
        selected_node = null;
        selected_cp = null;
        let selected_display = document.getElementById("selected-display");
        selected_display.innerHTML = "Selected: Train#" + selected_train + "<br/>";
        return;
    }

    selected_node = null;
    selected_train = null;
    selected_cp = null;
    let selected_display = document.getElementById("selected-display");
    selected_display.innerHTML = "Selected: None";
    event.preventDefault();
    return false;
});

main_canvas.addEventListener("contextmenu",
    event => {
        event.preventDefault();
        return false;
    });

// make window draggable
main_canvas.addEventListener("mousemove", event => {
    if (event.buttons === 1 && dragMode) {
        if (selected_node) {
            selected_node.x += event.movementX / 0.7;
            selected_node.y += event.movementY / 0.7;
            ctrl_socket.send("node_move\n" + selected_node.id + " " + selected_node.x + ";" + selected_node.y);
        } else if (selected_cp) {
            switch (selected_cp.type) {
                case "b2":
                    selected_cp.x += event.movementX / 0.7;
                    selected_cp.y += event.movementY / 0.7;
                    ctrl_socket.send("track_adjust\n" + selected_cp.track.id
                        + " " + selected_cp.x + ";" + selected_cp.y);
                    break;
                case "b31":
                    selected_cp.x += event.movementX / 0.7;
                    selected_cp.y += event.movementY / 0.7;
                    ctrl_socket.send("track_adjust\n" + selected_cp.track.id
                        + " " + selected_cp.x + ";" + selected_cp.y
                        + "," + selected_cp.track.cordlist[4] + ";" + selected_cp.track.cordlist[5]);
                    break;
                case "b32":
                    selected_cp.x += event.movementX / 0.7;
                    selected_cp.y += event.movementY / 0.7;
                    ctrl_socket.send("track_adjust\n" + selected_cp.track.id
                        + " " + selected_cp.track.cordlist[2] + ";" + selected_cp.track.cordlist[3]
                        + "," + selected_cp.x + ";" + selected_cp.y);
                    break;

            }
        } else {
            relative_x -= event.movementX / 0.7;
            relative_y -= event.movementY / 0.7;
            document.cookie = "relative_x=" + relative_x;
            document.cookie = "relative_y=" + relative_y;
        }
    }
});

main_canvas.addEventListener("mousemove", event => {
    let x = event.offsetX / 0.7 + relative_x;
    let y = event.offsetY / 0.7 + relative_y;
    window["mouse-pos-div"].innerText = "x: " + x.toFixed(0) + ", y: " + y.toFixed(0);
});
