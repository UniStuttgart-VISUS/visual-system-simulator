// Inject mockup Activity when running outside of Android app, e.g., for development.
var Activity;
if (typeof Activity === "undefined") {
    Activity = {
        querySettings: function () {
            let mockData = [{
                "MockNodeA": {
                    "MockParamA1": 42,
                    "MockParamA2": false
                }
            }, {
                "MockNodeB": {
                    "MockParamB1": "A String",
                    "MockParamB2": true
                }
            }];
            console.info("Querying Settings", mockData);
            return JSON.stringify(mockData);
        },
        postSettings: function (jsonString) {
            assert(typeof jsonString === "string");
            console.info("Posting Settings", JSON.parse(jsonString));
            return;
        }
    };
}

// Build DOM subtree with proper event listeners
function buildSettingsPanel(settings) {
    var elPanel = $(`<div class="settings-panel"></div>`);
    elPanel.settings = JSON.parse(JSON.stringify(settings));
    for (var flowIndex = 0; flowIndex < settings.length; ++flowIndex) {
        let flow = settings[flowIndex];
        for (var nodeName in flow) {
            elPanel.append(`<h6 id="sp__${nodeName}">${nodeName}</h6>`);
            let node = flow[nodeName];
            for (var attributeName in node) {
                let attributeValue = node[attributeName];
                let attributeId = `sp__${nodeName}__${attributeName}`;
                let attributeType;
                if (typeof attributeValue == "boolean") {
                    attributeType = "checkbox";
                } else if (typeof attributeValue == "number") {
                    attributeType = "number";
                } else {
                    attributeType = "text";
                }
                let elLabel = $(`<label for="${attributeId}">${attributeName}</label>`)
                let elInput = $(`<input type="${attributeType}" id="${attributeId}" name="${attributeId}"` +
                    `placeholder="Value" value="${attributeValue}"></input>`)
                elInput.on("input", function (flowIndex, nodeName, attributeName, attributeType) {
                    return function () {
                        var value;
                        if (attributeType == "checkbox") {
                            value = $(this).prop('checked');
                        } else if (attributeType == "number") {
                            value = parseFloat($(this).val());
                        } else {
                            value = $(this).val();
                        }
                        elPanel.settings[flowIndex][nodeName][attributeName] = value;
                        Activity.postSettings(JSON.stringify(elPanel.settings));
                    }
                }(flowIndex, nodeName, attributeName, attributeType));
                elLabel.append(elInput);
                elPanel.append(elLabel);
            }
        }
    }
    return elPanel;
}

$(function () {
    const settings = JSON.parse(Activity.querySettings());
    $("body").append(buildSettingsPanel(settings));
});
