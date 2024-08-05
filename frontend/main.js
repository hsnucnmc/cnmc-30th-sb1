let img = -1;
let image_name = "train_right.png";
const train_width = 350 / 4; // TODO: adjust with image size
const train_height = 263 / 4;

let relative_x = Number(document.cookie.split("; ").find(row => row.startsWith("relative_x="))?.split("=")[1]);
let relative_y = Number(document.cookie.split("; ").find(row => row.startsWith("relative_y="))?.split("=")[1]);

let ask_attempt = 0;

let debugMode = false;
let dragMode = true;

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

let status = "nothing";
let run_time = 1000.0; //?ms

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();
let explosionSerial = 0;
let explosionList = new Map();
let trainposition = [];

function drawRotatedImg(ctx, rotation_center_x, rotation_center_y, rotation_degree, object_x, object_y, img) {
    ctx.save();
    ctx.translate(rotation_center_x, rotation_center_y);
    ctx.rotate((Math.PI / 180) * rotation_degree);
    ctx.translate(-rotation_center_x, -rotation_center_y);
    ctx.drawImage(img, object_x, object_y, train_width, train_height);
    ctx.restore();
}

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

function drawPoint(ctx, point) {
    console.log(point);
    ctx.fillRect(point.x, point.y, 5, 5);
}

function drawSingleTie(ctx, track, length, current_t) {
    let pos = bezierPoint(track.cordlist, current_t);
    let un_normalized = bezierDerivative(track.cordlist, current_t);
    let speed = Math.sqrt(Math.pow(un_normalized.dx, 2) + Math.pow(un_normalized.dy, 2));
    let dx = un_normalized.dx / speed * length / 2;
    let dy = un_normalized.dy / speed * length / 2;
    ctx.beginPath();
    ctx.moveTo(pos.x + dy, pos.y - dx);
    ctx.lineTo(pos.x - dy, pos.y + dx);
    ctx.stroke();
}

function drawTrack(ctx, track) {
    let cordlist = track.cordlist;
    let color = track.color;
    let thickness = track.thickness;

    ctx.beginPath();
    ctx.strokeStyle = color;
    ctx.lineWidth = thickness;
    ctx.moveTo(cordlist[0], cordlist[1]);
    if (cordlist.length == 4) {
        ctx.lineTo(cordlist[2], cordlist[3]);
    } else if (cordlist.length == 6) {
        ctx.quadraticCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5]);
    } else {
        ctx.bezierCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5], cordlist[6], cordlist[7]);
    }
    ctx.stroke();

    ctx.beginPath();
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = thickness * 0.75;
    ctx.moveTo(cordlist[0], cordlist[1]);
    if (cordlist.length == 4) {
        ctx.lineTo(cordlist[2], cordlist[3]);
    } else if (cordlist.length == 6) {
        ctx.quadraticCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5]);
    } else {
        ctx.bezierCurveTo(cordlist[2], cordlist[3], cordlist[4], cordlist[5], cordlist[6], cordlist[7]);
    }
    ctx.stroke();

    // TODO: adjust tie density base off of track length
    ctx.strokeStyle = color;
    ctx.lineWidth = thickness / 6;
    let n = 30.0 * track.length / 500.0;
    for (let i = 0.5 / n / 2; i < 1; i += 1 / n) {
        drawSingleTie(ctx, track, thickness * 2, i);
    }
    if (debugMode) {
        ctx.strokeStyle = "#000";
        drawSingleTie(ctx, track, thickness * 2, 0);
    }
}

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

    if (img == -1) {//? default image
        if (!image_name) image_name = "train_right.png";
        img = new Image(); // Create new img element
        img.src = image_name;
    }

    main_context.fillStyle = "#BBB";
    main_context.fillStyle = "#BBB";
    nodelist.forEach(node => {
        main_context.beginPath();
        main_context.arc(node.x, node.y, 20, 0, 2 * Math.PI);
        main_context.fill();
    });

    tracklist.forEach(track => {
        drawTrack(main_context, track);
    });

    trainposition = [];

    explosionList.forEach((explosion, explosion_id) => {
        if (Number.isNaN(explosion.start)) {
            explosion.start = time;
        }

        switch (explosion.type) {
            case "e": // explosion
            case "d": // derail
                if (time - explosion.start > 5000) {
                    explosionList.delete(explosion_id);
                }
                break;
            case "v": // vibration
                {
                    if (time - explosion.start > 1000) {
                        explosionList.delete(explosion_id);
                    }
                    let x = (time - explosion.start) / 333;
                    let vibration_degree = 90 * Math.sin(20 / (x + 6 - 9.25) + 1) * (Math.pow(x,  0.5)/3);
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
        //! not handling out of bound problem
        // now detrive
        let dresult = bezierDerivative(cordlist, current_t);
        let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, train.img);
    });



    main_context.fillStyle = "#BBB";
    main_context.font = "40px monospace";
    nodelist.forEach(node => {
        main_context.fillText(node.id.toString().padStart(3, "0"), node.x - 20, node.y - 25);
    });

    main_context.restore();
    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");
let socket = null;
function startSocket() {
    socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");


    socket.onerror = e => {
        main_canvas.hidden = true;
        document.getElementById("main-canvas").parentElement.append(derail_img);
        window.setTimeout(startSocket, 500);
    };

    socket.onopen = (event) => {
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
        socket.onclose = msg => {
            main_canvas.hidden = true;
            document.getElementById("main-canvas").parentElement.append(derail_img);
            window.setTimeout(startSocket, 500);
        };
    };
}

startSocket();

// update click on demand
window.addEventListener("click", function (event) {
    mousePos = { x: event.clientX + relative_x, y: event.clientY + relative_y };
    r = Math.sqrt(Math.pow(train_width / 2, 2) + Math.pow(train_height / 2, 2));
    // time complexity (o(n))
    trainposition.forEach(pos => {
        clickr = Math.sqrt(Math.pow(mousePos.x - pos.x, 2) + Math.pow(mousePos.y - pos.y, 2));
        if (clickr <= r) {
            socket.send("click\n" + pos.id + " " + Number(event.ctrlKey) + "," + Number(event.shiftKey) + "," + Number(event.altKey));
        }
    });
});

// make window draggable
window.addEventListener("mousemove", event => {
    if (event.buttons === 1 && dragMode) {
        relative_x -= event.movementX;
        relative_y -= event.movementY;
        document.cookie = "relative_x=" + relative_x;
        document.cookie = "relative_y=" + relative_y;
    }
});