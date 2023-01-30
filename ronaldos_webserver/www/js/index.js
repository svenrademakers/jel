var fixtures = [];
class Match {
    constructor(league, fixture_id) {
        this.fixture_id = fixture_id;
        this.league = league;
    }

    get home() {
        return fixtures[this.league][this.fixture_id]["home"];
    }

    get away() {
        return fixtures[this.league][this.fixture_id]["away"];
    }

    get venue() {
        return fixtures[this.league][this.fixture_id]["venue"];
    }

    get score() {
        return fixtures[this.league][this.fixture_id]["score"];
    }

    get date() {
        let date = new Date(0);
        date.setUTCSeconds(fixtures[this.league][this.fixture_id]["timestamp"]);
        return "<div>" + $.format.date(date, "D MMM") + "</div><div>" + $.format.date(date, "HH:mm");
    }
}

function setup_match_table(title, id, watch, header_type = "h1") {
    output = `<${header_type}>${title}</${header_type}>`
    output += `<div class="row table table-responsive-md"> \
    <table class="table table-hover table-striped"> \
        <thead>\
            <tr>\
            <th scope="col">Date</th> \
            <th scope="col">Home</th> \
            <th scope="col">Away</th> \
            <th scope="col">Venue</th> \
            <th scope="col">Score</th>`;
    if (watch) {
        output += '<th scope="col">Watch</th>';
    }
    output += `</tr>\
        </thead>\
        <tbody id="${id}">\
        </tbody>\
    </table>\
</div>`;
    return output;
}

streams = []
fixtures = {}

$(document).ready(function () {
      $.get("streams/all", function (data) {
        $("#content-videos").append(setup_match_table("Videos", "schedule_table", true));
        streams = data;
        let output = "";
        for (const value of streams) {
            let btn_style = "btn-outline-primary";
            let btn_text = "Watch";

            if (value["live"]) {
                btn_style = "btn-outline-danger";
                btn_text = "Live";
                output += "<tr class='table-active'>";
            } else {
                output += "<tr>";
            }

            let date = new Date(0);
            date.setUTCSeconds(value["date"]);
            output += "<td scope='col'>" + $.format.date(date, "D MMM yyyy") + "</td>";
            output += "<td colspan=\"4\">" + value["description"] + "</td>";
            output += "<td><button type=\"button\" onclick=\"start_video('" + streams.indexOf(value) + "')\" class=\"btn " + btn_style + " \">" + btn_text + "</button></td>";
            output += "<tr>"
        }
        $("#schedule_table").html(output);

    });

    $.get("fixtures", function (data) {
        fixtures = data;
        $("#content").append("<h1>Fixtures</h1>")
        for (const [league, wat] of Object.entries(fixtures)) {
            let x = upcoming_fixture(league);
            let table = league.replaceAll(' ', '-') + '-table';
            $("#content").append(setup_match_table(league, table, false, "h2"));
            $(`#${table}`).html(schedule_table(league, x));
        }
    });

    options = {
            fluid: true,
            techOrder: [ 'chromecast', 'html5' ],
            html5: {
                vhs: {
                    overrideNative: true
                },
                nativeAudioTracks: false,
                nativeVideoTracks: false
            },
            autoplay: true,
            liveui: false,
    };
    var player = videojs('video_player', options, function () {
        this.chromecast();
        videojs.log("player loaded", this.currentSrc());
    });
})


function start_video(stream_id) {
    const stream = streams[stream_id];
    $("#current_title").text(stream["description"]);
    console.log(stream);

    if (stream["live"]) {
        $("#current_title").append("\t<span class=\"badge badge-danger\">Live</span>");
    }

    var video = videojs("video_player");
    let sources = []
    for (const source of stream["sources"]) {
        sources.push({
            type: source["typ"],
            src: source["url"],
        });
    }
    video.src(sources);
    video.play();

    if ($("#video-container").css('display') == 'none') {
        $("#video-container").fadeIn("slow");
    }

}

function schedule_table(league, index) {
    let output = "";
    for (let i = Math.max(0, index - 2); i < Object.entries(fixtures[league]).length; i++) {
        const match = new Match(league, Object.keys(fixtures[league])[i]);
        // disable previous fixtures
        if (i < index) {
            output += "<tr class=\"disabled\">";
        } else {
            output += "<tr>";
        }
        output += "<th scope=\"row\">" + match.date + "</th>";
        output += "<td>" + match.home +
            "</td>";
        output += "<td>" + match.away +
            "</td>";
        output += "<td>" + match.venue +
            "</td>";
        output += "<td>" + match.score +
            "</td>";
        output += "</tr>";
    }
    return output;
}

function upcoming_fixture(league) {
    // const index = Object.keys(fixtures[league]).findIndex(k => k === x);
    return Object.values(fixtures[league]).findIndex(fix => fix["timestamp"] * 1000 > Date.now());
}

function set_current_fixture(match) {
    let fixture = new Match(match);
    $("#current_title").text(fixture.home + " - " + fixture.away);
    // let start = new Date(0);
    // start.setUTCSeconds(fixtures[match]["timestamp"]);
    // start.setMinutes(start.getMinutes() - 15);

    // let end = start;
    // end.setHours(end.getHours() + 2);
    // let now = new Date();

    // if (now >= start && now < end) {
    //     $("#current_title").append(" <span class=\"badge badge-danger\">Live</span>");
    // video.src( {
    //     type: "application/x-mpegURL",
    //     src: "http://live.svenrademakers.com:8080/hls/" + fixture.fixture_id + ".m3u8",
    // });

    // var dash_source = $("<source>", {
    //     type: "application/dash+xml",
    //     "src": "http://live.svenrademakers.com:8080/dash/" + fixture.fixture_id + ".mpd",
    // });

    // } else {
    //     // countdown!
    //     $("#current_title").append("<span id=\"countdown\" class=\"float-right\"> </span>");
    //     $('#countdown').countdown(start, function (event) {
    //         $(this).html(event.strftime('\t%D days %H:%M:%S'));
    //     });
    // }
}

function start_countdown(match) {

}
