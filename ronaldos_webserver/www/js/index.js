var fixtures = [];
class Match {
    constructor(fixture_id) {
        this.fixture_id = fixture_id;
    }

    get home() {
        return fixtures[this.fixture_id]["home"];
    }

    get away() {
        return fixtures[this.fixture_id]["away"];
    }

    get venue() {
        return fixtures[this.fixture_id]["venue"];
    }

    get score() {
        return fixtures[this.fixture_id]["score"];
    }

    get date() {
        function get_date(date) {
            const nth = function (d) {
                const dString = String(d);
                const last = +dString.slice(-2);
                if (last > 3 && last < 21) return 'th';
                switch (last % 10) {
                    case 1:
                        return "st";
                    case 2:
                        return "nd";
                    case 3:
                        return "rd";
                    default:
                        return "th";
                }
            }
            let output = "<div>" + date.getMonth() + " " + date.getDay() + " " + nth(date.getDay()) + "</div>";
            output += "<div>" + date.getHours() + ":" + date.getMinutes() + "</div>";
            return output;
        }
        let date = new Date(0);
        date.setUTCSeconds(fixtures[this.fixture_id]["timestamp"]);
        return "<div>" + $.format.date(date, "D MMM") + "</div><div>" + $.format.date(date, "HH:mm");
    }
}

$(document).ready(function () {
    // $.get("fixtures", function (data) {
    //     fixtures = data;
    //     let x = upcoming_fixture();
    //     set_current_fixture(x);
    //     load_schedule_table(x);
    // });

    $.get("streams/all", function(data) {
        console.log(JSON.stringify(data));
        let output = "";
        for (const [key, value] of Object.entries(data)) {
            output += "<tr>";
            let date = new Date(0);
            date.setUTCSeconds(value["date"]);
            output += "<td>" + $.format.date(date, "D MMM yyyy") + "</td>";
            output += "<td>" + value["title"] + "</td>";
            output += "<td><button type=\"button\" onclick=\"set_video_src('" + value["sources"][0]["url"] + "')\" class=\"btn btn-outline-primary\">Watch</button></td>";
            output += "<tr>"
        }
        $("#schedule_table").html(output);

    });
    var player = videojs('video_player', {
        autoplay: true,
        liveui: false,
        inactivityTimeout: 0,
    }, function () {
        videojs.log("player loaded", this.currentSrc());
    });
})
function set_video_src(url) {
    var video = videojs("video_player");
    video.src(url);
}
function load_schedule_table(x) {
    const index = Object.keys(fixtures).findIndex(k => k === x);
    let output = "";
    for (let i = Math.max(0, index - 2); i < Object.entries(fixtures).length; i++) {
        const match = new Match(Object.keys(fixtures)[i]);
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
        if (has_watch(match.fixture_id)) {
            output += "<td><button type=\"button\" onclick=\"set_current_fixture(" + match.fixture_id + ")\" class=\"btn btn-outline-primary\">Watch</button></td>";
        }
        output += "</tr>";
    }
    $("#schedule_table").html(output);
}

function upcoming_fixture() {
    for (const [key, value] of Object.entries(fixtures)) {
        let game_time = fixtures[key]["timestamp"];
        if (game_time * 1000 > Date.now()) {
            return key;
        }
    }
}

function has_watch(fixt_id) {
    // return file_exists("matches/hls/".fixt_id.
    //         ".m3u8") ||
    //     file_exists("matches/dash/".fixt_id.
    //         ".mpd");
    return false;
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