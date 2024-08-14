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
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
let socket = null;
function startSocket() {
    socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");


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