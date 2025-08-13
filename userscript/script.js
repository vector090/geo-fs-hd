// ==UserScript==
// @name         Better Resolution Terrain dev
// @namespace    http://tampermonkey.net/
// @version      2024-01-21-2
// @description  Gets higher resolution images and blocks ads
// @author       drakeerv
// @match        https://www.geo-fs.com/geofs.php*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=geo-fs.com
// @grant        none
// @license      MIT
// @updateURL    https://raw.githubusercontent.com/vector090/geo-fs-hd/refs/heads/dev/userscript/script.js
// ==/UserScript==

(function() {
    "use strict";

    let viewer;

    // backup
    let tp;
    // <!-- geofs.api.FlatRunwayTerrainProvider -->

    function bing(){
        console.log("bing");
        viewer = geofs.api.viewer;
        viewer.imageryLayers.removeAll(true);

        // this is actually blank in geo-fs
        // may choose do this to ensure see google 3d
        // <!-- viewer.terrainProvider = Cesium.createWorldTerrain(); -->

        // this one has no heights
        // <!-- viewer.terrainProvider = new Cesium.EllipsoidTerrainProvider(); -->

        tp = viewer.terrainProvider;

        let bing2=new Cesium.BingMapsImageryProvider({
                        url: 'https://dev.virtualearth.net',
                        key: "AmXdbd8UeUJtaRSn7yVwyXgQlBBUqliLbHpgn2c76DfuHwAXfRrgS5qwfHU6Rhm8",
                        mapStyle: Cesium.BingMapsStyle.AERIAL_WITH_LABELS,
                        culture: 'zh-CN',
                    });
        viewer.imageryLayers.addImageryProvider(bing2);
    }


    let gtileset;

    function google(){
        console.log("google");
        const Maps_API_KEY = 'AIzaSyDwgHcF-bYuR1LtlN2MlnoeT_ac4Xl8Wis';
        const tilesetUrl = `https://tile.googleapis.com/v1/3dtiles/root.json?key=${Maps_API_KEY}`;

        if (!gtileset) {
            gtileset = new Cesium.Cesium3DTileset({
                url: tilesetUrl
            });
        }
        viewer.scene.primitives.add(gtileset);
    }

    function noGoogle(){
        console.log("noGoogle");
        viewer.scene.primitives.remove(gtileset);
    }

    let gEnabled = false;
    function toggleGoogle(){
        if(gEnabled){
            noGoogle();
            gEnabled = false;
        }else{
            google();
            gEnabled = true;
        }
    }

    function changeMap(){
        // console.log("changeMap");
        bing();
        // google();
    }

    setTimeout(changeMap, 8000);





    function slowAircraft(){
        console.log("slowAircraft");
        let xrate = 10;
        geofs.aircraft.instance.airfoils.forEach(i => { i.area *= xrate });

        // yet faster top speed
        geofs.aircraft.instance.definition.maxRPM=22222

        // optional
        geofs.aircraft.instance.definition.dragFactor=2
    }

    function sharperFlightPlan(){
        console.log("sharperFlightPlan");
        geofs.flightPlan.update = function (e) {
            if (!geofs.flightPlan.waypointArray.length)
                return;
            if (!geofs.flightPlan.trackedWaypoint) {
                geofs.flightPlan.selectWaypoint(0);
                return
            }
            if (geofs.flightPlan.trackedWaypoint.id == geofs.flightPlan.waypointArray.length - 1)
                return;
            let t = geofs.animation.getValue(geofs.nav.currentNAVUnitName + "DME")
                , a = 0.3; //0.6;//clamp(geofs.flightPlan.DMEMargin * geofs.aircraft.instance.llaLocation[2], 1, 100);
            console.log(t);
            t < a && geofs.flightPlan.selectWaypoint(geofs.flightPlan.trackedWaypoint.id + 1),
                geofs.flightPlan.isOpen && (geofs.flightPlan.distanceLeft = geofs.flightPlan.totalDistance - (geofs.flightPlan.trackedWaypoint.distanceThusfar - t),
                    $(".flightPlanDistance").html("(" + parseInt(geofs.flightPlan.distanceLeft) + "nm)"))
        }
    }

    function miscTunes(){
        // console.log("miscTunes");
        slowAircraft();
        sharperFlightPlan();
    }

    setTimeout(miscTunes, 10000);

    document.addEventListener('keydown', (event) => {
        if (event.shiftKey && event.key === 'G') {
            event.preventDefault();

            // console.log('Shift + G 组合键被按下！');
            toggleGoogle();
        }
    });

    // document.querySelectorAll("body > div.geofs-adbanner.geofs-adsense-container")[0].remove();
})();


