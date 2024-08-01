let img = -1;
let image_name = "train_right.png";
const train_width = 350 / 4; // TODO: adjust with image size
const train_height = 263 / 4;

let relative_x = Number(document.cookie.split("; ").find(row => row.startsWith("relative_x="))?.split("=")[1]);
let relative_y = Number(document.cookie.split("; ").find(row => row.startsWith("relative_y="))?.split("=")[1]);

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
derail_img.src = "derail.png";

let status = "nothing";
let movement_start = 0;
let run_time = 1000.0; //?ms

let trainlist = new Map();
let tracklist = new Map();
let trainposition = [];

function drawRotatedImg(ctx, rotation_center_x, rotation_center_y, rotation_degree, object_x, object_y, img) {
    ctx.save();
    ctx.translate(rotation_center_x, rotation_center_y);
    ctx.rotate((Math.PI / 180) * rotation_degree);
    ctx.translate(-rotation_center_x, -rotation_center_y);
    ctx.drawImage(img, object_x, object_y, train_width, train_height);
    ctx.restore();
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
    return {x: x_pos, y: y_pos};
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
        const invT2 = 2*t;
        const t2 = 2*t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * (-2) * t * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * (-2) * t * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { ddx: dx, ddy: dy };
    } else {
        throw new Error("Unsupported number of control points.\nSupported types are straight lines, quadratic (3 points) and cubic (4 points) beziers.");
    }
}

function radiusOfCurvature(cordlist, current_t) {
    let first = bezierDerivative(cordlist, current_t);
    let second = bezierSecondDerivative(cordlist, current_t);
    let dx = first.dx, dy = first.dy;
    let ddx = second.ddx, ddy = second.ddy;
    return Math.sqrt(dx*dx+dy*dy)/(dx*ddy-dy*ddy);
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
    ctx.moveTo(pos.x+dy, pos.y-dx);
    ctx.lineTo(pos.x-dy, pos.y+dx);
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
    ctx.lineWidth = thickness*0.75;
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
    let n = 30;
    for(let i = 0.5 / n / 2; i < 1; i+=1/n) {
        drawSingleTie(ctx, track, thickness * 2, i);
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
    main_context.translate(relative_x, relative_y);

    if (movement_start == -1) {
        movement_start = time;
    }

    if (img == -1) {//? default image
        if (!image_name) image_name = "train_right.png";
        img = new Image(); // Create new img element
        img.src = image_name;
    }

    tracklist.forEach(track => {
        drawTrack(main_context, track);
    });

    trainposition = [];
    trainlist.forEach((train, id) => {
        if (Number.isNaN(train.movement_start)) {
            train.movement_start = time - Number(train.start_t) * Number(train.duration);
        }

        let cordlist = tracklist.get(train.track_id).cordlist;
        let current_t = (time - train.movement_start) / train.duration;
        if (current_t > 1.1)
            return;
        let point = bezierPoint(cordlist, current_t);
        let x_pos = point.x;
        let y_pos = point.y;
        let trainpositionitem = {};
        trainpositionitem.id=id;
        trainpositionitem.x=x_pos;
        trainpositionitem.y=y_pos;
        trainposition.push(trainpositionitem);
        //! not handling out of bound problem
        // now detrive
        let dresult = bezierDerivative(cordlist, current_t);
        let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, train.img);
    });

    main_context.restore();
    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");
let socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");

socket.onopen = (event) => {
    // socket.send("position\n" + left_bound + " " + right_bound);
    socket.onmessage = (msg) => {
        // console.log(msg);
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
                new_train.img = new Image();
                new_train.img.src = msg_split[2];
                new_train.movement_start = NaN;

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

                    tracklist.set(Number(args[0]), track);
                }
                break;
        }
    };
    socket.onclose = (msg) => {
        main_canvas.hidden = true;
        document.getElementById("main-canvas").parentElement.append(derail_img);
    };
};

// update click on demand
window.addEventListener("click", function (event) {
    mousePos = { x: event.clientX, y: event.clientY };
    r=Math.sqrt(Math.pow(train_width/2,2)+Math.pow(train_height/2,2))
    // time complexity (o(n))
    trainposition.forEach(pos=>{
        clickr= Math.sqrt(Math.pow(mousePos.x-pos.x,2)+Math.pow(mousePos.y-pos.y,2));
        if(clickr<=r){
            socket.send("click\n" + pos.id);
        }
    })
});