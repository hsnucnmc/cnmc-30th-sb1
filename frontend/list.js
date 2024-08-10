let debugMode = false;

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();

let trainHTMLTable = document.getElementById("train-table");

let derail_img = new Image();
derail_img.id = "derail-img";
derail_img.src = "derail.png";

function pointDistance(p1, p2) {
    return Math.sqrt(Math.pow(p1.x - p2.x, 2) + Math.pow(p1.y - p2.y, 2));
}

function bezierPoint(cordlist, current_t) {
    let x_pos = 0, y_pos = 0;
    if (cordlist.length == 4) {
        // straight line
        x_pos = (1 - current_t) * cordlist[0] + current_t * cordlist[2];
        y_pos = (1 - current_t) * cordlist[1] + current_t * cordlist[3];
    } else if (cordlist.length == 6) {
        // one cp
        x_pos = (1 - current_t) * (1 - current_t) * cordlist[0] + 2 * (1 - current_t) * current_t * cordlist[2] + current_t * current_t * cordlist[4];
        y_pos = (1 - current_t) * (1 - current_t) * cordlist[1] + 2 * (1 - current_t) * current_t * cordlist[3] + current_t * current_t * cordlist[5];
    } else if (cordlist.length == 8) {
        // standard 2 cp
        x_pos = (1 - current_t) * (1 - current_t) * (1 - current_t) * cordlist[0] + 3 * (1 - current_t) * (1 - current_t) * current_t * cordlist[2] + 3 * (1 - current_t) * current_t * current_t * cordlist[4] + current_t * current_t * current_t * cordlist[6];
        y_pos = (1 - current_t) * (1 - current_t) * (1 - current_t) * cordlist[1] + 3 * (1 - current_t) * (1 - current_t) * current_t * cordlist[3] + 3 * (1 - current_t) * current_t * current_t * cordlist[5] + current_t * current_t * current_t * cordlist[7];
    }
    return { x: x_pos, y: y_pos };
}

function bezierDerivative(coords, t) {
    const n = (coords.length / 2) - 2; // Number of control points

    if (n === 0) {
        // Straight Line
        const [x0, y0, x1, y1] = coords;

        const invT = 1 - t;

        const dx = x1 - x0;
        const dy = y1 - y0;

        return { dx: dx, dy: dy };
    }
    else if (n === 1) {
        // Quadratic Bezier curve
        const [x0, y0, x1, y1, x2, y2] = coords;

        const invT = 1 - t;

        const dx = 2 * (1 - t) * (x1 - x0) + 2 * t * (x2 - x1);
        const dy = 2 * (1 - t) * (y1 - y0) + 2 * t * (y2 - y1);

        return { dx: dx, dy: dy };
    } else if (n === 2) {
        // Cubic Bezier curve
        const [x0, y0, x1, y1, x2, y2, x3, y3] = coords;

        const invT = 1 - t;
        const invT2 = invT * invT;
        const t2 = t * t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * t * invT * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * t * invT * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { dx: dx, dy: dy };
    } else {
        throw new Error("Unsupported number of control points.\nSupported types are straight lines, quadratic (3 points) and cubic (4 points) beziers.");
    }
}

function bezierSecondDerivative(coords, t) {
    const n = (coords.length / 2) - 2; // Number of control points

    if (n === 0) {
        // Straight Line

        return { ddx: 0, ddy: 0 };
    }
    else if (n === 1) {
        // Quadratic Bezier curve
        const [x0, y0, x1, y1, x2, y2] = coords;

        const invT = 1 - t;

        const dx = 2 * (-1) * (x1 - x0) + 2 * 1 * (x2 - x1);
        const dy = 2 * (-1) * (y1 - y0) + 2 * 1 * (y2 - y1);

        return { ddx: dx, ddy: dy };
    } else if (n === 2) {
        // Cubic Bezier curve
        const [x0, y0, x1, y1, x2, y2, x3, y3] = coords;

        const invT = -1;
        const invT2 = 2 * t;
        const t2 = 2 * t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * (-2) * t * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * (-2) * t * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { ddx: dx, ddy: dy };
    } else {
        throw new Error("Unsupported number of control points.\nSupported types are straight lines, quadratic (3 points) and cubic (4 points) beziers.");
    }
}

function bezierRoughLength(cordlist) {
    let length = 0;
    for (let i = 0; i <= 16; i++) {
        length += pointDistance(bezierPoint(cordlist, i / 16), bezierPoint(cordlist, (i + 1) / 16));
    }

    return length;
}

function radiusOfCurvature(cordlist, current_t) {
    let first = bezierDerivative(cordlist, current_t);
    let second = bezierSecondDerivative(cordlist, current_t);
    let dx = first.dx, dy = first.dy;
    let ddx = second.ddx, ddy = second.ddy;
    return Math.sqrt(dx * dx + dy * dy) / (dx * ddy - dy * ddy);
}

function redraw(time) {
    nodelist.forEach(node => {
        // main_context.beginPath();
        // main_context.arc(node.x, node.y, 20, 0, 2 * Math.PI);
        // main_context.fill();
    });

    tracklist.forEach(track => {
        // drawTrack(main_context, track);
    });

    trainlist.forEach((train, id) => {
        // if (Number.isNaN(train.movement_start)) {
        //     if (train.direction == 1) {
        //         train.movement_start = time - Number(train.start_t) * Number(train.duration);
        //     } else {
        //         train.movement_start = time - (1 - Number(train.start_t)) * Number(train.duration);
        //     }
        // }

        // let cordlist = tracklist.get(train.track_id).cordlist;
        // let current_t = (time - train.movement_start) / train.duration;
        // if (train.direction == -1) {
        //     current_t = 1 - current_t;
        // }
        // train.current_t = current_t;
        // if (current_t > 1.1 || current_t < -0.1)
        //     return;
        // let point = bezierPoint(cordlist, current_t);
        // let x_pos = point.x;
        // let y_pos = point.y;
        // let trainpositionitem = {};
        // train.x = x_pos;
        // train.y = y_pos;
        // //! not handling out of bound problem
        // // now detrive
        // let dresult = bezierDerivative(cordlist, current_t);
        // let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
    });

    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
let socket = null;
function startSocket() {
    socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");


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
        document.getElementById("status-container").innerText = "Connected! " + new Date();        ;

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
                    new_train.x = NaN;
                    new_train.y = NaN;

                    let new_row = trainHTMLTable.insertRow(-1);
                    new_row.insertCell(-1).innerText = Number(args[0]);
                    new_row.insertCell(-1).innerText = new_train.track_id;
                    if (args[4] == "forward") {
                        new_row.insertCell(-1).innerText = "=>";
                    } else {
                        new_row.insertCell(-1).innerText = "<=";
                    }
                    let row_img_src = document.createElement("pre");
                    row_img_src.innerText = new_train.img.src;
                    new_row.insertCell(-1).append(row_img_src);
                    new_row.insertCell(-1).innerText = "idk";
                    let row_progress = document.createElement("progress");
                    row_progress.value = 0.0;
                    new_row.insertCell(-1).append(row_progress);
                    train.html_row = new_row;
                    
                    trainlist.set(Number(args[0]), new_train);
                    break;
                case "track":
                    for (i = 2; i < msg_split.length; i++) {
                        args = msg_split[i].split(" ");
                        let track = {};
                        let cordlist = args[1].split(";").map(x => Number(x));
                        cordlist.shift();
                        track.cordlist = cordlist
                        track.color = args[2];
                        track.thickness = Number(args[3]);
                        track.length = bezierRoughLength(cordlist);

                        tracklist.set(Number(args[0]), track);
                    }
                    break;
                case "node":
                    args = msg_split[1].split(" ");
                    let new_node = {};
                    new_node.id = Number(args[0]);
                    new_node.x = Number(args[1].split(";")[0]);
                    new_node.y = Number(args[1].split(";")[1]);
                    nodelist.set(new_node.id, new_node);
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
