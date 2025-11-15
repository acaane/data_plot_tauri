<template>
    <div class="container">
        <h1 class="page-title">数据可视化</h1>
        <div class="header">
            <div class="controls">
                <input
                    v-model="logPath"
                    placeholder="请输入或选择文件路径"
                    class="path-input"
                />
                <button @click="selectFile" :disabled="selecting">
                    {{ selecting ? "选择中..." : "选择文件" }}
                </button>
                <button @click="loadData" :disabled="loading">
                    {{ loading ? '加载中...' : '加载数据' }}
                </button>
            </div>

            <div v-if="errMsg" class="error">
                {{ errMsg }}
            </div>
        </div>

        <div class="chart-container">
            <div ref="chart" class="chart"></div>
        </div>
    </div>
</template>

<script setup lang="ts">
interface TrainInfo {
    time: string,
    number: number | string,
    head: number | string,
    tail: number | string,
    height: number | string,
    pos: number | string,
}

enum LogType {
    None = 0,
    PingcheLog,
    MupianLog,
}

import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const chart = ref<HTMLDivElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const logPath = ref('../test_files/2025.10.31/pingche_log.5.log')
const selecting = ref(false)
const loading = ref(false)
const errMsg = ref('')
const timezoneOffset = 8 * 60 * 60 * 1000   // 当前时间戳与本地时间的偏差
let logType = LogType.None

// 转换时间字符串为时间戳
let parseTime = (time: string): number => {
    return new Date(time).getTime() - timezoneOffset
}

// 选择文件
async function selectFile() {
    try {
        selecting.value = true
        const selected = await open({
            filters: [{
                name: '日志',
                extensions: ['*']
            }],
            multiple: false
        })

        if (selected && typeof selected === 'string') {
            logPath.value = selected
            selecting.value = false

            // 获取文件名，并根据文件名判断日志对应项目
            let fileName = selected.split(/[\\/]/).pop() ?? ''
            console.log(fileName)
            if (fileName.includes('pingche_log')) {
                logType = LogType.PingcheLog
                errMsg.value = '平车机日志'
            } else if (fileName.includes('unload_log')) {
                logType = LogType.MupianLog
                errMsg.value = '木片小车日志'
            }
        }
    } catch (e: any) {
        errMsg.value = `选择文件失败:${e.message}`
    } finally {
        selecting.value = false
    }
}

async function loadPingcheData() {
    // 调用rust函数获取数据
    const data = await invoke('parse_data', {
        path: logPath.value.trim()
    })

    // 检查数据是否为空
    if (!data || Object.keys(data).length === 0) {
        errMsg.value = '未找到有效数据'
        loading.value = false
        return
    }

    // 转换为ECharts系列数据
    const series = Object
        .entries(data)
        .map(([trainNum, records]) => {
            // 按时间排序
            const sorted: [number, number, TrainInfo] = records
                .map((record: TrainInfo) => [
                    parseTime(record.time), // x轴数据
                    record.head,            // y轴数据
                    record                  // 要显示的数据
                ] as [number, number, TrainInfo])
                .sort((a: any, b: any) => a[0] - b[0])

            return {
                name: `车厢:${trainNum}`,
                type: 'line',
                data: sorted,
                smooth: true,
                showSymbol: false,
                emphasis: {
                    focus: 'series',
                    linsStyle: {
                        width: 4
                    },
                },
            }
        })

    // 配置图表选项
    const option: echarts.EChartsOption = {
        title: {
            text: '车厢位置-时间曲线',
            left: 'center',
            textStyle: {
                fontSize: 20,
                fontWeight: 'bold'
            }
        },
        tooltip: {
            trigger: 'axis',    // axis触发，同时出现当前时间线所有车厢数据
            axisPointer: {
                type: 'cross',
                animation: false,
                label: {
                    backgroundColor: "#6a7985"
                }
            },
            backgroundColor: 'rgba(255, 255, 255, 0.95)',
            borderColor: '#ccc',
            borderWidth: 1,
            textStyle: {
                color: '#333'
            },
            formatter: (params: any): string => {
                if (!params || params.length === 0) {
                    console.log('params为空')
                    return ''
                }

                const time = new Date(params[0].value[0]).toLocaleString('zh-CN')
                let result = `
                    <div style="
                        font-weight: bold;
                        margin-bottom: 8px;
                        padding-bottom: 5px;
                        border-bottom: 1px solid #eee;
                    ">
                        ${time}
                    </div>
                `
                params.forEach((param: any) => {
                    const trainNum = param.seriesName.replace('车厢:', '')
                    const info = param.value[2]
                    result += `
                        <div style="margin: 3px 0;">
                            <span style="
                                display: inline-block;
                                width: 10px;
                                height: 10px;
                                background: ${param.color};
                                border-radius: 50%;
                                margin-right: 8px;
                            ">
                            </span>
                            <strong>车厢</strong>${trainNum}
                            <strong>车头:</strong>${info.head.toFixed(2)}
                            <strong>车尾:</strong>${info.tail.toFixed(2)}
                            <strong>下铲高度:</strong>${info.height.toFixed(2)}
                            <strong>下铲位置:</strong>${info.pos.toFixed(2)}
                        </div>
                    `

                    // const [time, head, info] = param.value as [number, number, TrainInfo]
                    // const timeStr = new Date(time).toLocaleString('zh-CN')

                    // result += `
                    //     <div style="padding:6px;line-height:1.8;">
                    //     <div>时间：${timeStr}</div>
                    //     <div>车厢号：${info.number}</div>
                    //     <div>车头位置：${Number(info.head).toFixed(2)} m</div>
                    //     <div>车尾位置：${Number(info.tail).toFixed(2)} m</div>
                    //     <div>高度：${Number(info.height).toFixed(2)} m</div>
                    //     <div>POS：${Number(info.pos).toFixed(2)}</div>
                    //     </div>
                    // `
                })

                return result
            }
        },
        legend: {
            type: 'scroll',
            orient: 'vertical',
            right: 10,
            top: 60,
            bottom: 20,
            textStyle: {
                fontSize: 12
            },
            pageIconColor: '#409eff',
            pageTextStyle: {
                color: '#666'
            }
        },
        grid: {
            left: '3%',
            right: '15%',
            bottom: '15%',
            top: '15%',
            containLabel: true
        },
        toolbox: {
            feature: {
                dataZoom: {
                    yAxisIndex: 'none',
                    title: {
                        zoom: '区域缩放',
                        back: '缩放还原'
                    }
                },
                restore: {
                    title: '还原'
                },
                saveAsImage: {
                    title: '保存为图片',
                    backgroundColor: '#fff'
                }
            },
            right: 20,
            top: 20,
            iconStyle: {
                borderColor: '#409eff'
            },
            emphasis: {
                iconStyle: {
                    borderColor: '#66b1ff'
                }
            }
        },
        xAxis: {
            type: 'time',
            name: '时间',
            nameLocation: 'middle',
            nameGap: 30,
            boundaryGap: false,
            axisLine: {
                lineStyle: {
                    color: '#333'
                }
            },
            axisLabel: {
                formatter: (value: number) => {
                    const date = new Date(value)
                    const yy = date.getFullYear().toString().padStart(4, '0')
                    const MM = (date.getMonth() + 1).toString().padStart(2, '0')
                    const dd = (date.getDate()).toString().padStart(2, '0')
                    const hh = (date.getHours()).toString().padStart(2, '0')
                    const mm = (date.getMinutes()).toString().padStart(2, '0')
                    const ss = (date.getSeconds()).toString().padStart(2, '0')
                    return `${yy}/${MM}/${dd} ${hh}:${mm}:${ss}`
                },
                rotate: 45,
                color: '#666'
            }
        } as any,
        yAxis: {
            type: "value",
            name: '位置(m)',
            nameLocation: 'middle',
            nameGap: 50,
            axisLine: {
                lineStyle: {
                    color: '#333'
                }
            },
            axisLabel: {
                color: '#666'
            }
        },
        dataZoom: [
            {
                type: 'inside',
                start: 0,
                end: 100
            },
            {
                start: 0,
                end: 100,
                height: 30,
                bottom: 20,
                handleStyle: {
                    color: '#409eff'
                },
                textStyle: {
                    color: '#666'
                },
            }
        ],
        series: series.map(s => ({
            ...s,
            smooth: true,
            showSymbol: false
        })) as any
    }

    // 渲染图表
    if (!chartInstance) {
        chartInstance = echarts.init(chart.value!)
    }

    chartInstance.setOption(option, true)
}

async function loadMupianData() {
    console.log('loadMupianData')
}

async function loadData() {
    if (!logPath.value.trim()) {
        errMsg.value = '请输入或选择文件路径'
        return
    }

    loading.value = true
    errMsg.value = ''

    try {
        if (logType === LogType.PingcheLog) {
            loadPingcheData()
        } else if (logType === LogType.MupianLog) {
            loadMupianData()
        }

        // 响应式调整
        window.addEventListener('resize', () => {
            chartInstance?.resize()
        })
    } catch (e: any) {
        console.error(`加载数据失败:${e}`)
        errMsg.value = `加载数据失败:${e.message || e}`
    } finally {
        loading.value = false
    }
}

// 组件加载时初始化图表
onMounted(() => {
    chartInstance = echarts.init(chart.value!)

    // 可以自动加载数据，或等待用户点击
})

// 组件卸载时销毁图标
onUnmounted(() => {
    if (chartInstance) {
        chartInstance.dispose()
        chartInstance = null
    }
})

</script>

<style scoped>
.container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 20px;
    box-sizing: border-box;
}

.page-title {
    margin: 0 0 20px 0;
    font-size: 35px;
    font-weight: 600;
    text-align: center;
    width: 100%;
    color: #333;
}

.header {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 1px solid #e0e0e0;
}

.controls {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: center;
    width: 100%;
}

.path-input {
    width: 400px;
    padding: 8px 12px;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    font-size: 14px;
    transition: border-color 0.3s;
}

.path-input:focus {
    outline: none;
    border-color: #409eff;
}

button {
    padding: 8px 16px;
    background-color: #409eff;
    color: white; 
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    transition: background-color 0.3s;
}

button:hover:not(:disabled) {
    background-color: #66b1ff;
}

button:disabled {
    background-color: #a0cfff;
    cursor: not-allowed;
}

.chart-container {
    width: 100%;
    height: 100%;       /* 父级必须有高度 */
    min-height: 400px;  /* 兜底 */
}

.chart {
    width: 100%;
    height: 100%;
}

.error {
    margin-top: 8px;
    padding: 12px;
    background-color: #fef0f0;
    border: 1px solid #fbc4c4;
    border-radius: 4px;
    color: #f56c6c;
    font-size: 14px;
}
</style>
