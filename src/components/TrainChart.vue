<script setup lang="ts">
import { error } from 'echarts/types/src/util/log.js';

</script>

<template>
    <div class="container">
        <h1 class="page-title">平车日志可视化</h1>
        <div class="header">
            <div class="controls">
                <input
                    v-model="logPath"
                    placeholder="请输入或选择日志文件路径"
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

<script lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog';

const chart = ref<HTMLDivElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const logPath = ref('D:/project/pingche_jingtang/data/log/2025.10.31/pingche_log.5.log')
const loading = ref(false)
const selecting = ref(false)
const errMsg = ref('')

// 转换ISO时间字符串为时间戳
function parseTime(time: string): number {
    return new Date(time).getTime()
}

// 选择文件
async function selectFile() {
    try {
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
        }
    } catch (e: any) {
        errMsg.value = `选择文件失败:${e.message}`
    }
}

// 加载并渲染数据
async function loadData() {
    if (!logPath.value.trim()) {
        errMsg.value = '请输入日志文件路径'
        return
    }

    loading.value = true
    errMsg.value = ''

    try {
        // 调用rust函数获取数据
        const data:Record<string, any[]> = await invoke('parse_data', {
            path: logPath.value.trim()
        })

        // 检查数据是否为空
        if (!data || Object.keys(data).length === 0) {
            errMsg.value = '未找到有效数据'
            loading.value = false
            return
        }

        // 转换为ECharts系列数据
        const series = Object.entries(data).map(([trainNum, records]) => {
            // 按时间排序
            const sortedRecords = records
                .map((record: any) => [parseTime(record.time), Number(record.head)] as [number, number])
                .sort((a, b) => a[0] - b[0])

            return {
                name: `车厢${trainNum}`,
                type: 'line',
                data: sortedRecords,
                smooth: true,
                showSymbol: false,
                emphasis: {
                    focus: 'series',
                    lineStyle: {
                        width: 4
                    }
                }
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
                trigger: 'axis',
                axisPointer: {
                    type: 'cross',
                    animation: false,
                    label: {
                        backgroundColor: '#6a7985'
                    }
                } as any,
                backgroundColor: 'rgba(255, 255, 255, 0.95)',
                borderColor: '#ccc',
                borderWidth: 1,
                textStyle: {
                    color: '#333'
                },
                formatter: (params: any): string => {
                    if (!params || params.length === 0) return ''
                    
                    const time = new Date(params[0].value[0]).toLocaleString('zh-CN')
                    let result = `<div style="font-weight: bold; margin-bottom: 8px; padding-bottom: 5px; border-bottom: 1px solid #eee;">${time}</div>`
                    
                    params.forEach((param: any) => {
                        const trainNum = param.seriesName.replace('车厢 ', '')
                        const pos = param.value[1]
                        result += `<div style="margin: 3px 0;">
                        <span style="display: inline-block; width: 10px; height: 10px; 
                            background: ${param.color}; border-radius: 50%; margin-right: 8px;"></span>
                        车厢${trainNum}: ${pos.toFixed(2)} m
                        </div>`
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
                    formatter: function(value: number) {
                        const date = new Date(value)
                        const hh = date.getHours().toString().padStart(2, '0')
                        const mm = date.getMinutes().toString().padStart(2, '0')
                        const ss = date.getSeconds().toString().padStart(2, '0')
                        return `${hh}:${mm}:${ss}`
                    },
                    rotate: 45,
                    color: '#666'
                } as any
            } as any,
            yAxis: {
                type: 'value',
                name: '位置 (m)',
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
                    }
                }
            ],
            series: series.map(s => ({ ...s, smooth: true, showSymbol: false })) as any
        }

        // 渲染图表
        if (!chartInstance) {
            chartInstance = echarts.init(chart.value!)
        }
        
        chartInstance.setOption(option, true)
        
        // 响应式调整
        window.addEventListener('resize', () => {
            chartInstance?.resize()
        })
    } catch (err: any) {
        console.error('加载数据失败:', err)
        errMsg.value = `加载数据失败: ${err.message || err}`
    } finally {
        loading.value = false
    }
}

// 组件挂载时初始化图表
onMounted(() => {
    chartInstance = echarts.init(chart.value!)
    
    // 可以自动加载数据，或等待用户点击
    // loadData()
})

// 组件卸载时销毁图表
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
  text-align: center;        /* 文字水平居中 */
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
  height: 100%;      /* 父级必须有高度 */
  min-height: 400px; /* 兜底 */
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
